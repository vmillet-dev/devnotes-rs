use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::model::NotePatch;

use super::pasted_item;
use crate::notes::bench::{Bench, area, click_id, settle};
use crate::testing;

type Harness = egui_kittest::Harness<'static, Bench>;

fn item(text: &str, done: bool) -> ChecklistItem {
    ChecklistItem {
        text: text.into(),
        done,
    }
}

/// A list holding `items`, open in the editor.
fn list(items: &[ChecklistItem]) -> (Harness, String) {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "groceries").id.clone();
    let patch = NotePatch {
        kind: Some(NoteKind::Checklist),
        content: Some(String::new()),
        items: Some(items.to_vec()),
        ..NotePatch::default()
    };
    let at = id.clone();
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::update_note(engine, at, patch)
    })
    .unwrap();
    let mut harness = area(backend, profile);
    // A list's card is mostly its ticks: it opens as Enter on the card opens it.
    harness.state_mut().state.editor.open(id.clone());
    settle(&mut harness);
    (harness, id)
}

fn stored(harness: &Harness, id: &str) -> Vec<(String, bool)> {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap()
    .items
    .clone()
    .into_iter()
    .map(|item| (item.text, item.done))
    .collect()
}

fn rows(harness: &Harness) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some("checklist-text"))
        .map(|node| node.value().unwrap_or_default())
        .collect()
}

fn focus_row(harness: &mut Harness, index: usize) {
    harness
        .query_all_by(|node| node.author_id() == Some("checklist-text"))
        .nth(index)
        .unwrap()
        .click();
    settle(harness);
}

fn press(harness: &mut Harness, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    settle(harness);
}

fn owned(pairs: &[(&str, bool)]) -> Vec<(String, bool)> {
    pairs
        .iter()
        .map(|(text, done)| ((*text).to_owned(), *done))
        .collect()
}

#[test]
fn a_task_added_and_typed_is_written_on_close_and_a_tick_at_once() {
    let (mut harness, id) = list(&[item("milk", false)]);

    click_id(&mut harness, "checklist-add");
    harness
        .query_all_by(|node| node.author_id() == Some("checklist-text"))
        .nth(1)
        .unwrap()
        .type_text("bread");
    settle(&mut harness);
    click_id(&mut harness, "editor-close");
    assert_eq!(
        stored(&harness, &id),
        owned(&[("milk", false), ("bread", false)])
    );

    harness.state_mut().state.editor.open(id.clone());
    settle(&mut harness);
    harness
        .query_all_by(|node| node.author_id() == Some("checklist-check"))
        .nth(1)
        .unwrap()
        .click();
    settle(&mut harness);
    assert_eq!(
        stored(&harness, &id),
        owned(&[("milk", false), ("bread", true)])
    );
    let progress = harness
        .get_by(|node| node.author_id() == Some("checklist-progress"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert!(progress.starts_with("1/"), "{progress}");
}

#[test]
fn enter_splits_a_task_at_the_caret_and_backspace_joins_it_back() {
    let (mut harness, id) = list(&[item("milkbread", false)]);
    focus_row(&mut harness, 0);
    press(&mut harness, Modifiers::NONE, Key::End);
    for _ in 0..5 {
        press(&mut harness, Modifiers::NONE, Key::ArrowLeft);
    }
    press(&mut harness, Modifiers::NONE, Key::Enter);

    assert_eq!(rows(&harness), ["milk", "bread"]);
    assert_eq!(
        stored(&harness, &id),
        owned(&[("milk", false), ("bread", false)])
    );

    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(rows(&harness), ["milkbread"]);
}

#[test]
fn backspace_in_an_empty_task_removes_it_and_the_cross_does_too() {
    // The core keeps no empty task: one is added here.
    let (mut harness, id) = list(&[item("milk", false), item("eggs", false)]);
    click_id(&mut harness, "checklist-add");
    assert_eq!(rows(&harness), ["milk", "eggs", ""]);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(rows(&harness), ["milk", "eggs"]);

    harness
        .query_all_by(|node| node.author_id() == Some("checklist-remove"))
        .nth(1)
        .unwrap()
        .click();
    settle(&mut harness);
    assert_eq!(stored(&harness, &id), owned(&[("milk", false)]));
}

#[test]
fn lines_pasted_are_tasks_their_boxes_read() {
    let (mut harness, id) = list(&[item("x", false)]);
    click_id(&mut harness, "checklist-add");
    harness.event(Event::Paste(
        "- [x] ship it\n\n- [ ] tag it\n3. announce".into(),
    ));
    settle(&mut harness);

    assert_eq!(
        stored(&harness, &id),
        owned(&[
            ("x", false),
            ("ship it", true),
            ("tag it", false),
            ("announce", false)
        ])
    );
}

#[test]
fn alt_arrows_move_a_task_and_the_grip_drags_it() {
    let (mut harness, id) = list(&[item("a", false), item("b", false), item("c", false)]);
    focus_row(&mut harness, 0);
    press(&mut harness, Modifiers::ALT, Key::ArrowDown);
    assert_eq!(
        stored(&harness, &id),
        owned(&[("b", false), ("a", false), ("c", false)])
    );

    let grips: Vec<egui::Rect> = harness
        .query_all_by(|node| node.author_id() == Some("checklist-grip"))
        .map(|node| node.rect())
        .collect();
    let (from, to) = (grips[0].center(), grips[2].center());
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0] {
        harness.hover_at(from + (to - from) * step);
        harness.run();
    }
    harness.drop_at(to);
    settle(&mut harness);

    assert_eq!(
        stored(&harness, &id),
        owned(&[("a", false), ("c", false), ("b", false)])
    );
}

#[test]
fn a_pasted_line_reads_its_marker_and_its_box() {
    assert_eq!(pasted_item("  - [X] done "), item("done", true));
    assert_eq!(pasted_item("1) first"), item("first", false));
    assert_eq!(pasted_item("* [ ] open"), item("open", false));
    assert_eq!(pasted_item("plain line"), item("plain line", false));
    assert_eq!(pasted_item("-dash"), item("dash", false));
}
