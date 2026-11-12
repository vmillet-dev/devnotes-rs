//! What the explorer reads of a view the core answered, never of the text: the selection, what
//! holds it, where it was written. No path is ever taken apart here.

use engine::json::model::{JsonGraph, JsonNode, JsonRow, JsonView};
use engine::json::parse::Span;

/// What a selection is: a node, or a value in one of its rows.
#[derive(Debug, Clone, Copy)]
pub(super) enum Found<'a> {
    Node(&'a JsonNode),
    Row {
        node: &'a JsonNode,
        row: &'a JsonRow,
        index: usize,
    },
}

impl<'a> Found<'a> {
    pub(super) fn node(self) -> &'a JsonNode {
        match self {
            Found::Node(node) | Found::Row { node, .. } => node,
        }
    }
}

pub(super) fn find<'a>(graph: &'a JsonGraph, path: &str) -> Option<Found<'a>> {
    if let Some(node) = graph.nodes.iter().find(|node| node.path == path) {
        return Some(Found::Node(node));
    }
    graph.nodes.iter().find_map(|node| {
        let index = node.rows.iter().position(|row| row.path == path)?;
        Some(Found::Row {
            node,
            row: &node.rows[index],
            index,
        })
    })
}

pub(super) fn parent<'a>(graph: &'a JsonGraph, node: &JsonNode) -> Option<&'a JsonNode> {
    let at = usize::try_from(node.parent?).ok()?;
    graph.nodes.get(at)
}

pub(super) fn child<'a>(graph: &'a JsonGraph, row: &JsonRow) -> Option<&'a JsonNode> {
    let at = usize::try_from(row.child?).ok()?;
    graph.nodes.get(at)
}

/// The node and its ancestors, root first: the breadcrumb, and the path the graph lights.
pub(super) fn lineage<'a>(graph: &'a JsonGraph, node: &'a JsonNode) -> Vec<&'a JsonNode> {
    let mut chain = vec![node];
    while let Some(up) = parent(graph, chain[chain.len() - 1]) {
        chain.push(up);
    }
    chain.reverse();
    chain
}

/// Whether the view draws the entry: a node or a row of the graph, or a line of the tree.
pub(super) fn shows(view: &JsonView, path: &str) -> bool {
    find(&view.graph, path).is_some() || view.tree.iter().any(|line| line.path == path)
}

/// The entry and what holds it, outermost first, read from the graph's parents or the tree's
/// depths.
pub(super) fn trail_of(view: &JsonView, path: &str) -> Vec<String> {
    if let Some(found) = find(&view.graph, path) {
        let mut chain: Vec<String> = lineage(&view.graph, found.node())
            .into_iter()
            .map(|node| node.path.clone())
            .collect();
        if let Found::Row { row, .. } = found {
            chain.push(row.path.clone());
        }
        return chain;
    }
    let Some(at) = view.tree.iter().position(|line| line.path == path) else {
        return vec![path.to_owned()];
    };
    let mut trail = vec![path.to_owned()];
    let mut depth = view.tree[at].depth;
    for line in view.tree[..at].iter().rev() {
        if depth == 0 {
            break;
        }
        if line.depth < depth {
            trail.push(line.path.clone());
            depth = line.depth;
        }
    }
    trail.reverse();
    trail
}

/// One level of the value, its containers folded: what the selection panel previews.
pub(super) fn preview(node: &JsonNode) -> String {
    use engine::json::model::JsonKind;
    let object = node.kind == JsonKind::Object;
    let mut entries: Vec<String> = node
        .rows
        .iter()
        .map(|row| {
            let value = match row.kind {
                JsonKind::Object => "{ … }",
                JsonKind::Array => "[ … ]",
                _ => row.value.as_str(),
            };
            if object {
                format!("  {}: {value}", quoted(&row.key))
            } else {
                format!("  {value}")
            }
        })
        .collect();
    if node.hidden_rows > 0 {
        entries.push("  …".to_owned());
    }
    let (open, close) = if object { ('{', '}') } else { ('[', ']') };
    if entries.is_empty() {
        format!("{open}{close}")
    } else {
        format!("{open}\n{}\n{close}", entries.join(",\n"))
    }
}

fn quoted(key: &str) -> String {
    serde_json::to_string(key).unwrap_or_else(|_| format!("\"{key}\""))
}

/// Where the entry was written: a node, a row of one, or a line of the tree.
pub fn span_of(view: &JsonView, path: &str) -> Option<Span> {
    match find(&view.graph, path) {
        Some(Found::Node(node)) => Some(node.span),
        Some(Found::Row { row, .. }) => Some(row.span),
        None => view
            .tree
            .iter()
            .find(|line| line.path == path)
            .map(|line| line.span),
    }
}

/// The innermost entry the view draws around a UTF-16 offset of the text: what the caret is in.
pub fn entry_at(view: &JsonView, offset: u32) -> Option<&str> {
    let lines = view.tree.iter().map(|line| (line.path.as_str(), line.span));
    let nodes = view.graph.nodes.iter().flat_map(|node| {
        std::iter::once((node.path.as_str(), node.span))
            .chain(node.rows.iter().map(|row| (row.path.as_str(), row.span)))
    });
    lines
        .chain(nodes)
        .filter(|(_, span)| span.start <= offset && offset <= span.end)
        .min_by_key(|(_, span)| span.end - span.start)
        .map(|(path, _)| path)
}

/// The text a span covers, its UTF-16 units read back into the string; empty past its end.
pub fn utf16_slice(text: &str, span: Span) -> &str {
    let byte = |units: u32| {
        let mut counted = 0;
        for (at, glyph) in text.char_indices() {
            if counted >= units {
                return at;
            }
            counted += u32::try_from(glyph.len_utf16()).unwrap_or(2);
        }
        text.len()
    };
    let (start, end) = (byte(span.start), byte(span.end));
    text.get(start..end.max(start)).unwrap_or_default()
}
