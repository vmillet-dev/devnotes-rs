//! What the JSON visualiser shows, decided here: the front draws it.

use std::collections::{BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use specta::Type;

use super::parse::{JsonError, Node, Span, Value, parse};
use crate::notes::view::fold;

/// Past this, arrays are folded and the tree is proposed: every node sent is one drawn.
pub const MAX_NODES: usize = 2000;
/// What a graph may draw in all, rows included: one array of fifty thousand numbers is one node.
const MAX_ROWS: usize = 20_000;
/// A node longer than this shows its first rows and says how many it left out.
const ROWS_PER_NODE: usize = 200;
/// A document this small opens whole; a larger one opens on its first level.
const OPEN_WHOLE_UP_TO: usize = 64;
const MAX_TREE_LINES: usize = 5000;
const MAX_MATCHES: usize = 1000;
/// A value is shown on one row, cut to this many characters.
const VALUE_CHARS: usize = 40;

/// Grid units: a column is a character of the monospace font, a row one line of a node.
const COLUMN_GAP: u32 = 6;
const NODE_GAP: u32 = 1;
const MIN_WIDTH: u32 = 16;
const MAX_WIDTH: u32 = 48;

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum JsonOpening {
    /// Whole when small, its first level otherwise.
    Initial,
    All,
    /// `JSONPath`s, the root being always open: a path whose parent is closed stays closed.
    Paths {
        paths: Vec<String>,
    },
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonQuery {
    pub text: String,
    pub search: String,
    pub opening: JsonOpening,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum JsonKind {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonRow {
    /// `id`, or `[0]` in an array.
    pub key: String,
    pub path: String,
    pub kind: JsonKind,
    /// `"paid"`, `4900`, `{ … }`, `[ 3 ]`: cut to one line.
    pub value: String,
    pub span: Span,
    /// The node drawn for this value, when it is a container and open.
    pub child: Option<u32>,
    /// Whether the value is a container, open or not.
    pub opens: bool,
    pub hit: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonNode {
    pub id: u32,
    pub parent: Option<u32>,
    pub path: String,
    /// `$`, the key it is under, or `[0]`.
    pub label: String,
    pub kind: JsonKind,
    /// Keys or items, all of them: `rows` may hold fewer.
    pub size: u32,
    pub rows: Vec<JsonRow>,
    /// Past `ROWS_PER_NODE`.
    pub hidden_rows: u32,
    pub span: Span,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    /// The header line, one per row, one more for the rows left out.
    pub height: u32,
    pub hit: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonGraph {
    pub nodes: Vec<JsonNode>,
    pub width: u32,
    pub height: u32,
    /// Opening everything asked was more than the front can draw: arrays stayed folded.
    pub folded: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonLine {
    pub depth: u32,
    pub key: String,
    pub path: String,
    pub kind: JsonKind,
    pub value: String,
    pub size: u32,
    pub span: Span,
    pub opens: bool,
    pub open: bool,
    pub hit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonStats {
    pub keys: u32,
    /// Container levels below the root: `{"a": {"b": 1}}` is 1.
    pub depth: u32,
    pub bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JsonView {
    /// `None` is "JSON valide"; the rest is then empty.
    pub error: Option<JsonError>,
    pub stats: JsonStats,
    pub graph: JsonGraph,
    pub tree: Vec<JsonLine>,
    pub tree_truncated: bool,
    /// Every entry the search found, in the document's order, open or not.
    pub matches: Vec<String>,
    pub match_count: u32,
}

pub fn explore(query: &JsonQuery) -> JsonView {
    let bytes = count(query.text.len());
    let root = match parse(&query.text) {
        Ok(root) => root,
        Err(error) => return JsonView::invalid(error, bytes),
    };

    let needle = fold(query.search.trim());
    let entries = Entry::flatten(&root, &needle);
    let (open, folded) = opened(&entries, &query.opening);
    let hits: Vec<&Entry> = entries.iter().filter(|entry| entry.hit).collect();
    let (tree, tree_truncated) = tree(&entries, &open);

    JsonView {
        error: None,
        stats: stats(&entries, bytes),
        graph: graph(&entries, &open, folded),
        tree,
        tree_truncated,
        matches: hits
            .iter()
            .take(MAX_MATCHES)
            .map(|entry| entry.path.clone())
            .collect(),
        match_count: count(hits.len()),
    }
}

impl JsonView {
    fn invalid(error: JsonError, bytes: u32) -> Self {
        Self {
            error: Some(error),
            stats: JsonStats {
                keys: 0,
                depth: 0,
                bytes,
            },
            graph: JsonGraph {
                nodes: Vec::new(),
                width: 0,
                height: 0,
                folded: false,
            },
            tree: Vec::new(),
            tree_truncated: false,
            matches: Vec::new(),
            match_count: 0,
        }
    }
}

/// Every value of the document, in its order, flattened once: the graph, the tree, the search
/// and the counts all read this.
struct Entry {
    parent: Option<usize>,
    depth: u32,
    key: String,
    path: String,
    kind: JsonKind,
    value: String,
    size: u32,
    span: Span,
    in_array: bool,
    is_key: bool,
    children: Vec<usize>,
    hit: bool,
}

impl Entry {
    fn flatten(root: &Node, needle: &str) -> Vec<Self> {
        let mut entries = Vec::new();
        visit(
            root,
            None,
            "$".to_string(),
            "$".to_string(),
            false,
            &mut entries,
            needle,
        );
        entries
    }

    fn container(&self) -> bool {
        matches!(self.kind, JsonKind::Object | JsonKind::Array)
    }
}

fn visit(
    node: &Node,
    parent: Option<usize>,
    key: String,
    path: String,
    in_array: bool,
    entries: &mut Vec<Entry>,
    needle: &str,
) {
    let index = entries.len();
    let depth = parent.map_or(0, |parent| entries[parent].depth + 1);
    let (kind, value, size, searched) = describe(&node.value);
    let hit = parent.is_some()
        && !needle.is_empty()
        && (fold(&key).contains(needle)
            || searched.is_some_and(|text| fold(text).contains(needle)));

    entries.push(Entry {
        parent,
        depth,
        key,
        path,
        kind,
        value,
        size,
        span: node.span,
        in_array,
        is_key: parent.is_some() && !in_array,
        children: Vec::new(),
        hit,
    });
    if let Some(parent) = parent {
        entries[parent].children.push(index);
    }

    match &node.value {
        Value::Object(members) => {
            for (name, child) in members {
                let path = member_path(&entries[index].path, name);
                visit(
                    child,
                    Some(index),
                    name.clone(),
                    path,
                    false,
                    entries,
                    needle,
                );
            }
        }
        Value::Array(items) => {
            for (position, child) in items.iter().enumerate() {
                let label = format!("[{position}]");
                let path = format!("{}{label}", entries[index].path);
                visit(child, Some(index), label, path, true, entries, needle);
            }
        }
        _ => {}
    }
}

/// The kind, what a row shows, the size, and what the search reads of a scalar.
fn describe(value: &Value) -> (JsonKind, String, u32, Option<&str>) {
    match value {
        Value::Object(members) => (
            JsonKind::Object,
            "{ … }".to_string(),
            count(members.len()),
            None,
        ),
        Value::Array(items) => (
            JsonKind::Array,
            format!("[ {} ]", items.len()),
            count(items.len()),
            None,
        ),
        Value::String(text) => (
            JsonKind::String,
            format!("\"{}\"", cut(text)),
            0,
            Some(text),
        ),
        Value::Number(raw) => (JsonKind::Number, cut(raw), 0, Some(raw)),
        Value::Bool(true) => (JsonKind::Boolean, "true".to_string(), 0, Some("true")),
        Value::Bool(false) => (JsonKind::Boolean, "false".to_string(), 0, Some("false")),
        Value::Null => (JsonKind::Null, "null".to_string(), 0, Some("null")),
    }
}

/// One line, however many the value had: a row is one line high.
fn cut(text: &str) -> String {
    let line: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(VALUE_CHARS + 1)
        .collect();
    if line.chars().count() > VALUE_CHARS {
        format!(
            "{}…",
            line.chars().take(VALUE_CHARS - 1).collect::<String>()
        )
    } else {
        line
    }
}

/// `$.data.lines`, or `$['a key']` for a key that is no identifier.
fn member_path(parent: &str, key: &str) -> String {
    let identifier = key
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if identifier {
        format!("{parent}.{key}")
    } else {
        format!(
            "{parent}['{}']",
            key.replace('\\', "\\\\").replace('\'', "\\'")
        )
    }
}

fn stats(entries: &[Entry], bytes: u32) -> JsonStats {
    let keys = entries.iter().filter(|entry| entry.is_key).count();
    let depth = entries
        .iter()
        .filter(|entry| entry.container())
        .map(|entry| entry.depth)
        .max()
        .unwrap_or(0);

    JsonStats {
        keys: count(keys),
        depth,
        bytes,
    }
}

/// Which containers are open, in breadth-first order so a cap keeps the levels nearest the
/// root, and whether the cap folded any.
fn opened(entries: &[Entry], opening: &JsonOpening) -> (BTreeSet<usize>, bool) {
    let containers = entries.iter().filter(|entry| entry.container()).count();
    let asked: Option<BTreeSet<&str>> = match opening {
        JsonOpening::Paths { paths } => Some(paths.iter().map(String::as_str).collect()),
        _ => None,
    };
    let wants = |index: usize, entry: &Entry| -> bool {
        match opening {
            JsonOpening::All => true,
            JsonOpening::Initial => containers <= OPEN_WHOLE_UP_TO || entry.depth <= 1,
            JsonOpening::Paths { .. } => {
                index == 0
                    || asked
                        .as_ref()
                        .is_some_and(|asked| asked.contains(entry.path.as_str()))
            }
        }
    };

    let breadth_first = |skip_array_items: bool| -> (BTreeSet<usize>, bool) {
        let mut open = BTreeSet::new();
        let mut rows = 0;
        let mut capped = false;
        let mut queue = VecDeque::from([0]);
        while let Some(index) = queue.pop_front() {
            let entry = &entries[index];
            if !entry.container() || !wants(index, entry) {
                continue;
            }
            if skip_array_items && entry.in_array && index != 0 {
                capped = true;
                continue;
            }
            let shown = entry.children.len().min(ROWS_PER_NODE);
            if open.len() == MAX_NODES || rows + shown > MAX_ROWS {
                capped = true;
                continue;
            }
            open.insert(index);
            rows += shown;
            queue.extend(entry.children.iter().copied().take(ROWS_PER_NODE));
        }
        (open, capped)
    };

    let (open, capped) = breadth_first(false);
    if capped && matches!(opening, JsonOpening::All) {
        let (folded, _) = breadth_first(true);
        return (folded, true);
    }
    (open, capped)
}

fn graph(entries: &[Entry], open: &BTreeSet<usize>, folded: bool) -> JsonGraph {
    let ids: Vec<usize> = open.iter().copied().collect();
    let id_of = |index: usize| ids.binary_search(&index).ok().map(count);

    let mut nodes: Vec<JsonNode> = ids
        .iter()
        .map(|&index| {
            let entry = &entries[index];
            let rows: Vec<JsonRow> = entry
                .children
                .iter()
                .take(ROWS_PER_NODE)
                .map(|&child| {
                    let row = &entries[child];
                    JsonRow {
                        key: row.key.clone(),
                        path: row.path.clone(),
                        kind: row.kind,
                        value: row.value.clone(),
                        span: row.span,
                        child: id_of(child),
                        opens: row.container(),
                        hit: row.hit,
                    }
                })
                .collect();
            let hidden_rows = count(entry.children.len().saturating_sub(ROWS_PER_NODE));

            JsonNode {
                id: id_of(index).unwrap_or_default(),
                parent: entry.parent.and_then(id_of),
                path: entry.path.clone(),
                label: entry.key.clone(),
                kind: entry.kind,
                size: entry.size,
                width: width(entry, &rows),
                height: 1 + count(rows.len()) + u32::from(hidden_rows > 0),
                rows,
                hidden_rows,
                span: entry.span,
                x: 0,
                y: 0,
                hit: entry.hit,
            }
        })
        .collect();

    let (width, height) = super::layout::place(&mut nodes, COLUMN_GAP, NODE_GAP);
    JsonGraph {
        nodes,
        width,
        height,
        folded,
    }
}

/// Wide enough for its widest row, within bounds: the front cuts what still overflows.
fn width(entry: &Entry, rows: &[JsonRow]) -> u32 {
    let header = entry.key.chars().count() + 8;
    let widest = rows
        .iter()
        .map(|row| row.key.chars().count() + row.value.chars().count() + 4)
        .max()
        .unwrap_or(0);

    count(header.max(widest)).clamp(MIN_WIDTH, MAX_WIDTH)
}

fn tree(entries: &[Entry], open: &BTreeSet<usize>) -> (Vec<JsonLine>, bool) {
    let mut lines = Vec::new();
    let mut stack = vec![0];
    while let Some(index) = stack.pop() {
        if lines.len() == MAX_TREE_LINES {
            return (lines, true);
        }
        let entry = &entries[index];
        let is_open = open.contains(&index);
        lines.push(JsonLine {
            depth: entry.depth,
            key: entry.key.clone(),
            path: entry.path.clone(),
            kind: entry.kind,
            value: entry.value.clone(),
            size: entry.size,
            span: entry.span,
            opens: entry.container(),
            open: is_open,
            hit: entry.hit,
        });
        if is_open {
            stack.extend(entry.children.iter().rev());
        }
    }
    (lines, false)
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVOICE: &str = r#"{
  "id": "evt_1PqX4c",
  "type": "invoice.paid",
  "livemode": false,
  "data": {
    "object": {
      "amount_paid": 4900,
      "customer": { "name": "Ana Duval", "email": "ana@exemple.fr" },
      "lines": [
        { "description": "Pro · mensuel", "period": { "start": 1, "end": 2 } },
        { "description": "Stockage" }
      ]
    }
  }
}"#;

    fn query(text: &str, search: &str, opening: JsonOpening) -> JsonQuery {
        JsonQuery {
            text: text.to_string(),
            search: search.to_string(),
            opening,
        }
    }

    fn explored(text: &str) -> JsonView {
        explore(&query(text, "", JsonOpening::Initial))
    }

    fn node<'a>(view: &'a JsonView, path: &str) -> &'a JsonNode {
        view.graph
            .nodes
            .iter()
            .find(|node| node.path == path)
            .unwrap_or_else(|| panic!("no node at {path}"))
    }

    /// `n` objects in one array: a document that is not small.
    fn many(n: usize) -> String {
        let items: Vec<String> = (0..n).map(|i| format!(r#"{{"i": {i}}}"#)).collect();
        format!("[{}]", items.join(","))
    }

    #[test]
    fn a_small_document_opens_whole() {
        let view = explored(INVOICE);
        let paths: Vec<&str> = view
            .graph
            .nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect();

        assert_eq!(
            paths,
            [
                "$",
                "$.data",
                "$.data.object",
                "$.data.object.customer",
                "$.data.object.lines",
                "$.data.object.lines[0]",
                "$.data.object.lines[0].period",
                "$.data.object.lines[1]",
            ]
        );
        assert!(view.error.is_none());
        assert!(!view.graph.folded);
    }

    #[test]
    fn a_node_lists_its_values_as_rows_and_opens_its_containers() {
        let view = explored(INVOICE);
        let root = node(&view, "$");
        let values: Vec<(&str, &str, JsonKind)> = root
            .rows
            .iter()
            .map(|row| (row.key.as_str(), row.value.as_str(), row.kind))
            .collect();

        assert_eq!(
            values,
            [
                ("id", "\"evt_1PqX4c\"", JsonKind::String),
                ("type", "\"invoice.paid\"", JsonKind::String),
                ("livemode", "false", JsonKind::Boolean),
                ("data", "{ … }", JsonKind::Object),
            ]
        );
        assert_eq!(
            (root.label.as_str(), root.kind, root.size),
            ("$", JsonKind::Object, 4)
        );
        assert_eq!(root.rows[3].child, Some(node(&view, "$.data").id));
        assert!(root.rows[3].opens && !root.rows[0].opens);

        let lines = node(&view, "$.data.object.lines");
        assert_eq!(
            (lines.label.as_str(), lines.kind, lines.size),
            ("lines", JsonKind::Array, 2)
        );
        assert_eq!(node(&view, "$.data.object.lines[0]").label, "[0]");
        assert_eq!(node(&view, "$.data").parent, Some(root.id));
    }

    #[test]
    fn a_value_keeps_where_it_is_in_the_text() {
        let view = explored(INVOICE);
        let row = &node(&view, "$").rows[0];

        assert_eq!(
            &INVOICE[row.span.start as usize..row.span.end as usize],
            "\"evt_1PqX4c\""
        );
    }

    #[test]
    fn counts_the_keys_the_depth_and_the_bytes() {
        let stats = explored(INVOICE).stats;

        assert_eq!(stats.keys, 15);
        assert_eq!(stats.depth, 5);
        assert_eq!(stats.bytes, u32::try_from(INVOICE.len()).unwrap());
    }

    #[test]
    fn a_larger_document_opens_on_its_first_level() {
        let view = explored(&many(100));

        assert_eq!(view.graph.nodes.len(), 101);
        assert!(
            view.graph.nodes[1..]
                .iter()
                .all(|node| node.rows[0].child.is_none())
        );
        assert!(!view.graph.folded);
    }

    #[test]
    fn opens_the_paths_asked_and_none_under_a_closed_one() {
        let opening = JsonOpening::Paths {
            paths: vec!["$.data".into(), "$.data.object.customer".into()],
        };
        let view = explore(&query(INVOICE, "", opening));
        let paths: Vec<&str> = view
            .graph
            .nodes
            .iter()
            .map(|node| node.path.as_str())
            .collect();

        assert_eq!(paths, ["$", "$.data"]);
        assert_eq!(node(&view, "$.data").rows[0].child, None);
    }

    #[test]
    fn opening_everything_folds_arrays_past_what_can_be_drawn() {
        // Twenty arrays of 150 objects: 3,021 containers, the arrays each within a node's rows.
        let arrays: Vec<String> = (0..20)
            .map(|i| format!(r#""a{i}": {}"#, many(150)))
            .collect();
        let view = explore(&query(
            &format!("{{{}}}", arrays.join(",")),
            "",
            JsonOpening::All,
        ));

        assert!(view.graph.folded);
        assert_eq!(view.graph.nodes.len(), 21);
        assert!(
            view.graph.nodes[1]
                .rows
                .iter()
                .all(|row| row.opens && row.child.is_none())
        );
    }

    #[test]
    fn only_the_rows_shown_can_open() {
        let view = explore(&query(&many(MAX_NODES + 50), "", JsonOpening::All));

        assert!(!view.graph.folded);
        assert_eq!(view.graph.nodes.len(), 1 + ROWS_PER_NODE);
    }

    #[test]
    fn a_long_array_shows_its_first_rows_and_counts_the_rest() {
        let numbers: Vec<String> = (0..500).map(|i| i.to_string()).collect();
        let view = explored(&format!("[{}]", numbers.join(",")));
        let root = node(&view, "$");

        assert_eq!(root.rows.len(), ROWS_PER_NODE);
        assert_eq!(root.hidden_rows, 300);
        assert_eq!(root.height, 1 + 200 + 1);
    }

    #[test]
    fn finds_keys_and_values_folded_like_every_search() {
        let view = explore(&query(INVOICE, "mensuel", JsonOpening::Initial));

        assert_eq!(view.matches, ["$.data.object.lines[0].description"]);
        assert_eq!(view.match_count, 1);
        assert!(node(&view, "$.data.object.lines[0]").rows[0].hit);

        let accents = explore(&query(INVOICE, "EXEMPLE", JsonOpening::Initial));
        assert_eq!(accents.matches, ["$.data.object.customer.email"]);

        let keys = explore(&query(INVOICE, "period", JsonOpening::Initial));
        assert_eq!(keys.matches, ["$.data.object.lines[0].period"]);
        assert!(node(&keys, "$.data.object.lines[0].period").hit);
    }

    #[test]
    fn finds_what_is_not_open_too() {
        let opening = JsonOpening::Paths { paths: Vec::new() };
        let view = explore(&query(INVOICE, "stockage", opening));

        assert_eq!(view.matches, ["$.data.object.lines[1].description"]);
        assert_eq!(view.graph.nodes.len(), 1);
    }

    #[test]
    fn places_levels_left_to_right_and_children_by_their_rows() {
        let view = explored(INVOICE);
        let root = node(&view, "$");
        let data = node(&view, "$.data");
        let customer = node(&view, "$.data.object.customer");
        let lines = node(&view, "$.data.object.lines");

        assert_eq!(root.x, 0);
        assert_eq!(data.x, root.width + COLUMN_GAP);
        // `data` is the root's fourth row, under its header line.
        assert_eq!(data.y, root.y + 1 + 3);
        assert_eq!(customer.x, lines.x);
        assert!(lines.y >= customer.y + customer.height + NODE_GAP);
        assert!(view.graph.width >= lines.x + lines.width);
        assert!(view.graph.height >= lines.y + lines.height);
    }

    #[test]
    fn sizes_a_node_to_its_widest_row_within_bounds() {
        let view = explored(r#"{"a": 1}"#);
        assert_eq!(node(&view, "$").width, MIN_WIDTH);

        let wide = explored(&format!(r#"{{"key": "{}"}}"#, "x".repeat(200)));
        assert_eq!(node(&wide, "$").width, MAX_WIDTH);
        assert_eq!(
            node(&wide, "$").rows[0].value.chars().count(),
            VALUE_CHARS + 2
        );
    }

    #[test]
    fn a_value_on_several_lines_is_shown_on_one() {
        let view = explored(r#"["a\nb"]"#);

        assert_eq!(node(&view, "$").rows[0].value, "\"a b\"");
    }

    #[test]
    fn a_key_that_is_no_identifier_is_quoted_in_its_path() {
        let view = explored(r#"{"a key": {"it's": {"ok_1": {}}}}"#);

        assert!(
            view.graph
                .nodes
                .iter()
                .any(|node| node.path == r"$['a key']['it\'s'].ok_1")
        );
    }

    #[test]
    fn the_tree_lists_what_is_open_in_the_documents_order() {
        let opening = JsonOpening::Paths {
            paths: vec!["$.data".into()],
        };
        let view = explore(&query(INVOICE, "", opening));
        let lines: Vec<(u32, &str, bool, bool)> = view
            .tree
            .iter()
            .map(|line| (line.depth, line.key.as_str(), line.opens, line.open))
            .collect();

        assert_eq!(
            lines,
            [
                (0, "$", true, true),
                (1, "id", false, false),
                (1, "type", false, false),
                (1, "livemode", false, false),
                (1, "data", true, true),
                (2, "object", true, false),
            ]
        );
        assert!(!view.tree_truncated);
    }

    #[test]
    fn the_tree_stops_at_what_a_list_can_hold() {
        let numbers: Vec<String> = (0..6000).map(|i| i.to_string()).collect();
        let all = explore(&query(
            &format!("[{}]", numbers.join(",")),
            "",
            JsonOpening::All,
        ));

        assert_eq!(all.tree.len(), MAX_TREE_LINES);
        assert!(all.tree_truncated);
    }

    #[test]
    fn an_invalid_document_says_where_and_draws_nothing() {
        let view = explored("{\"a\": }");

        let error = view.error.expect("an error");
        assert_eq!((error.line, error.column), (1, 7));
        assert!(view.graph.nodes.is_empty() && view.tree.is_empty());
        assert_eq!(view.stats.bytes, 7);
    }

    #[test]
    fn a_scalar_document_is_one_line_and_no_node() {
        let view = explored("42");

        assert!(view.graph.nodes.is_empty());
        assert_eq!(view.tree.len(), 1);
        assert_eq!((view.graph.width, view.graph.height), (0, 0));
    }
}
