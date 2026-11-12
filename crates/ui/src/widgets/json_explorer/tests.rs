//! The explorer's reads over views the core answered, as the reference's model specs had them.

use engine::json::model::{JsonOpening, JsonQuery, JsonView, explore};
use engine::json::parse::Span;
use engine::json::read::StructureFormat;

use super::reads::{find, preview, trail_of};
use super::{Explorer, entry_at, span_of, utf16_slice};

const EVENT: &str =
    r#"{"id":"evt_1","data":{"object":{"amount":4900,"lines":[{"description":"Pro · mensuel"}]}}}"#;

fn view(text: &str, opening: JsonOpening) -> JsonView {
    explore(&JsonQuery {
        text: text.to_owned(),
        format: StructureFormat::Json,
        search: String::new(),
        opening,
    })
}

#[test]
fn a_row_is_found_in_its_node_and_its_trail_reads_the_parents() {
    let view = view(EVENT, JsonOpening::All);
    let found = find(&view.graph, "$.data.object.amount").unwrap();
    assert_eq!(found.node().path, "$.data.object");
    assert_eq!(
        trail_of(&view, "$.data.object.amount"),
        ["$", "$.data", "$.data.object", "$.data.object.amount"]
    );
}

#[test]
fn the_tree_s_depths_give_the_trail_of_a_line_the_graph_does_not_draw() {
    let mut view = view(EVENT, JsonOpening::All);
    view.graph.nodes.clear();
    assert_eq!(
        trail_of(&view, "$.data.object.lines"),
        ["$", "$.data", "$.data.object", "$.data.object.lines"]
    );
}

#[test]
fn the_innermost_entry_holds_the_caret() {
    let view = view(EVENT, JsonOpening::All);
    let at = EVENT.find("Pro").unwrap();
    let offset = u32::try_from(EVENT[..at].encode_utf16().count()).unwrap();
    assert_eq!(
        entry_at(&view, offset),
        Some("$.data.object.lines[0].description")
    );
    let span = span_of(&view, "$.data.object.lines[0].description").unwrap();
    assert_eq!(utf16_slice(EVENT, span), "\"Pro · mensuel\"");
}

#[test]
fn a_span_is_read_back_in_utf16_units_past_a_wide_character() {
    let text = "🦀: \"a\"";
    assert_eq!(utf16_slice(text, Span { start: 4, end: 7 }), "\"a\"");
    assert_eq!(utf16_slice(text, Span { start: 9, end: 12 }), "");
}

#[test]
fn the_preview_shows_one_level_its_containers_folded() {
    let view = view(EVENT, JsonOpening::All);
    let data = view
        .graph
        .nodes
        .iter()
        .find(|node| node.path == "$.data.object")
        .unwrap();
    assert_eq!(
        preview(data),
        "{\n  \"amount\": 4900,\n  \"lines\": [ … ]\n}"
    );
}

#[test]
fn folded_away_the_selection_falls_back_to_what_held_it_and_comes_back() {
    let open = view(EVENT, JsonOpening::All);
    let mut explorer = Explorer::default();
    explorer.select("$.data.object.amount");
    explorer.collapse_all(&open);
    let folded = view(
        EVENT,
        JsonOpening::Paths {
            paths: Vec::new(),
            reveal: None,
        },
    );
    assert_eq!(explorer.selected_path(&folded).as_deref(), Some("$.data"));
    assert_eq!(
        explorer.selected_path(&open).as_deref(),
        Some("$.data.object.amount")
    );
}

#[test]
fn stepping_to_a_match_reveals_it_and_wraps_around() {
    let mut explorer = Explorer::default();
    let found = explore(&JsonQuery {
        text: r#"{"a":"x","b":{"c":"x"}}"#.to_owned(),
        format: StructureFormat::Json,
        search: "x".to_owned(),
        opening: JsonOpening::Initial,
    });
    assert_eq!(explorer.step(&found, false).as_deref(), Some("$.b.c"));
    assert_eq!(explorer.step(&found, true).as_deref(), Some("$.a"));
    let asked = explorer.query(String::new(), StructureFormat::Json);
    assert!(matches!(
        asked.opening,
        JsonOpening::Paths { reveal: Some(path), .. } if path == "$.a"
    ));
}
