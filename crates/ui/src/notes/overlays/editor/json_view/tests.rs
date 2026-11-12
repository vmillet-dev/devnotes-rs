use egui::{Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::language::Language;
use engine::notes::model::NotePatch;

use crate::notes::bench::{Bench, click_id, expanded_area, settle, until};
use crate::testing;

type Harness<'a> = egui_kittest::Harness<'a, Bench>;

const EVENT: &str = r#"{
  "id": "evt_1",
  "type": "invoice.paid",
  "data": {
    "object": {
      "amount": 4900,
      "lines": [
        { "description": "Pro · mensuel" }
      ]
    }
  }
}"#;

fn snippet(content: &str, language: Language) -> Harness<'static> {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "webhook").id.clone();
    let patch = NotePatch {
        content: Some(content.into()),
        language: Some(language),
        ..NotePatch::default()
    };
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, id, patch)
    })
    .unwrap();
    let mut harness = expanded_area(backend, profile);
    click_id(&mut harness, "note-card-open");
    until(&mut harness, "the draft explored", |harness| {
        has(harness, "editor-json-validity")
    });
    harness
}

fn has(harness: &Harness<'_>, author_id: &str) -> bool {
    count(harness, author_id) > 0
}

fn count(harness: &Harness<'_>, author_id: &str) -> usize {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .count()
}

/// The entry of the graph or the tree written at `path`.
fn click_at(harness: &mut Harness<'_>, author_id: &str, path: &str) {
    harness
        .get_by(|node| {
            node.author_id() == Some(author_id) && node.description().as_deref() == Some(path)
        })
        .click();
    settle(harness);
}

/// What a control says: its name, or its value for a label.
fn said(harness: &Harness<'_>, author_id: &str) -> String {
    let node = harness.get_by(|node| node.author_id() == Some(author_id));
    let node = node.accesskit_node();
    node.label().or_else(|| node.value()).unwrap_or_default()
}

fn selected(harness: &Harness<'_>) -> String {
    said(harness, "json-selected-path")
}

fn code_selection(harness: &Harness<'_>) -> String {
    let view = harness.state().state.notes_view.editor.as_ref().unwrap();
    view.code.as_ref().unwrap().editor.buffer.selected_text()
}

fn press(harness: &mut Harness<'_>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    settle(harness);
}

fn graph(harness: &mut Harness<'_>) {
    click_id(harness, "editor-view-graph");
    until(harness, "the nodes drawn", |harness| {
        has(harness, "json-node-head")
    });
}

#[test]
fn a_json_snippet_parses_and_shows_its_graph_and_a_node_s_path() {
    let mut harness = snippet(EVENT, Language::Json);
    let validity = harness.get_by(|node| node.author_id() == Some("editor-json-validity"));
    assert!(validity.accesskit_node().data().invalid().is_none());

    graph(&mut harness);
    click_id(&mut harness, "json-fit");
    assert_eq!(count(&harness, "json-node-head"), 5);
    click_at(&mut harness, "json-node-head", "$.data.object");
    assert_eq!(selected(&harness), "$.data.object");
    let crumbs = said(&harness, "json-crumbs");
    assert!(crumbs.ends_with("object"), "{crumbs}");

    click_at(&mut harness, "json-node-head", "$");
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(selected(&harness), "$.id", "the arrows walk the rows");
}

#[test]
fn a_row_closes_and_opens_its_container_one_level() {
    let mut harness = snippet(EVENT, Language::Json);
    graph(&mut harness);
    click_at(&mut harness, "json-row", "$.data.object.lines");
    until(&mut harness, "the lines closed", |harness| {
        count(harness, "json-node-head") == 3
    });
    click_at(&mut harness, "json-row", "$.data.object.lines");
    until(&mut harness, "the lines open again", |harness| {
        count(harness, "json-node-head") == 4
    });
}

#[test]
fn folded_to_the_root_the_selection_falls_back_then_comes_back() {
    let mut harness = snippet(EVENT, Language::Json);
    graph(&mut harness);
    click_id(&mut harness, "json-fit");
    click_at(&mut harness, "json-node-head", "$.data.object");
    click_id(&mut harness, "json-collapse-all");
    until(&mut harness, "the root alone", |harness| {
        count(harness, "json-node-head") == 1
    });
    assert_eq!(selected(&harness), "$.data");

    click_id(&mut harness, "json-open-all");
    until(&mut harness, "every node again", |harness| {
        count(harness, "json-node-head") == 5
    });
    assert_eq!(selected(&harness), "$.data.object");

    click_at(&mut harness, "json-node-head", "$");
    press(&mut harness, Modifiers::SHIFT, Key::ArrowLeft);
    until(&mut harness, "the root alone, from the keys", |harness| {
        count(harness, "json-node-head") == 1
    });
    press(&mut harness, Modifiers::SHIFT, Key::ArrowRight);
    until(&mut harness, "every node, from the keys", |harness| {
        count(harness, "json-node-head") == 5
    });
}

#[test]
fn a_node_moved_with_alt_and_an_arrow_is_put_back_by_the_reset() {
    let mut harness = snippet(EVENT, Language::Json);
    graph(&mut harness);
    let top = |harness: &Harness<'_>| {
        harness
            .get_by(|node| {
                node.author_id() == Some("json-node-head")
                    && node.description().as_deref() == Some("$.data")
            })
            .rect()
            .top()
    };
    let start = top(&harness);
    click_at(&mut harness, "json-node-head", "$.data");
    press(&mut harness, Modifiers::ALT, Key::ArrowDown);
    assert!((top(&harness) - start - 24.0).abs() < 0.5, "a line lower");
    click_id(&mut harness, "json-reset-layout");
    assert!((top(&harness) - start).abs() < 0.5);
    assert!(!has(&harness, "json-reset-layout"));
}

#[test]
fn a_match_found_is_stepped_to_and_shown_in_the_code() {
    let mut harness = snippet(EVENT, Language::Json);
    graph(&mut harness);
    harness
        .get_by(|node| node.author_id() == Some("json-search"))
        .focus();
    harness.step();
    harness
        .get_by(|node| node.author_id() == Some("json-search"))
        .type_text("MENSUEL");
    until(&mut harness, "one match", |harness| {
        has(harness, "json-match-count") && said(harness, "json-match-count") == "1"
    });
    click_id(&mut harness, "json-match-next");
    until(&mut harness, "the match selected", |harness| {
        selected(harness) == "$.data.object.lines[0].description"
    });

    click_id(&mut harness, "json-show-in-code");
    assert!(has(&harness, "editor-body"));
    assert_eq!(code_selection(&harness), "\"Pro · mensuel\"");
}

#[test]
fn the_tree_folds_a_container_clicked() {
    let mut harness = snippet(EVENT, Language::Json);
    click_id(&mut harness, "editor-view-tree");
    until(&mut harness, "the lines drawn", |harness| {
        has(harness, "json-line")
    });
    let lines = count(&harness, "json-line");
    click_at(&mut harness, "json-line", "$.data");
    until(&mut harness, "the tree folded", |harness| {
        count(harness, "json-line") < lines
    });
}

#[test]
fn side_by_side_each_selects_in_the_other_and_the_code_hides() {
    let mut harness = snippet(EVENT, Language::Json);
    click_id(&mut harness, "editor-side-by-side");
    until(&mut harness, "the graph beside the code", |harness| {
        has(harness, "side-by-side") && has(harness, "json-node-head")
    });
    assert!(has(&harness, "side-by-side-code"));

    click_at(&mut harness, "json-node-head", "$.data");
    let lit = code_selection(&harness);
    assert!(lit.starts_with("{\n    \"object\": {"), "{lit}");

    click_id(&mut harness, "editor-body");
    press(&mut harness, Modifiers::COMMAND, Key::Home);
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    press(&mut harness, Modifiers::NONE, Key::End);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    assert_eq!(selected(&harness), "$.id", "the value under the caret");

    click_id(&mut harness, "editor-hide-code");
    assert!(!has(&harness, "side-by-side"));
    assert!(has(&harness, "json-node-head"));
}

#[test]
fn a_broken_draft_says_where_it_breaks() {
    let mut harness = snippet("{\"a\": 1,,}", Language::Json);
    until(&mut harness, "the draft refused", |harness| {
        harness
            .get_by(|node| node.author_id() == Some("editor-json-validity"))
            .accesskit_node()
            .data()
            .invalid()
            .is_some()
    });
    click_id(&mut harness, "editor-view-graph");
    until(&mut harness, "the place said", |harness| {
        has(harness, "json-invalid")
    });
    click_id(&mut harness, "json-error-show");
    assert!(has(&harness, "editor-body"));
}

#[test]
fn a_yaml_snippet_is_explored_and_plain_text_is_not() {
    let mut harness = snippet("services:\n  api:\n    image: \"api:1.4\"\n", Language::Yml);
    graph(&mut harness);
    assert!(has(&harness, "json-format-note"));

    let harness = {
        let (backend, profile) = testing::unlocked();
        testing::note(&backend, "page");
        let mut harness = expanded_area(backend, profile);
        click_id(&mut harness, "note-card-open");
        harness
    };
    assert!(!has(&harness, "editor-view-graph"));
}
