use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::kind::NoteKind;
use engine::notes::model::DisplayNote;

use crate::notes::bench::{Bench, area, cards, click_id, settle};
use crate::testing;

fn stored(harness: &egui_kittest::Harness<'_, Bench>, id: &str) -> DisplayNote {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, id)
    })
    .unwrap()
}

fn written(harness: &egui_kittest::Harness<'_, Bench>) -> usize {
    harness
        .state()
        .state
        .notes_query
        .view()
        .unwrap()
        .cards()
        .count()
}

fn is_open(harness: &egui_kittest::Harness<'_, Bench>) -> bool {
    harness
        .query_by(|node| node.author_id() == Some("editor-title"))
        .is_some()
}

/// The menu's entry, not the rail's row of the same name under the editor.
fn menu_entry(harness: &mut egui_kittest::Harness<'_, Bench>, name: &str) {
    harness
        .get_by(|node| {
            node.role() == egui::accesskit::Role::MenuItem && node.label().as_deref() == Some(name)
        })
        .click();
    settle(harness);
}

fn type_into(harness: &mut egui_kittest::Harness<'_, Bench>, id: &str, text: &str) {
    click_id(harness, id);
    harness
        .get_by(|node| node.author_id() == Some(id))
        .type_text(text);
    settle(harness);
}

#[test]
fn a_note_opened_edited_and_closed_shows_its_change_on_its_card() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql prod").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-body", "\nSELECT 2;");
    click_id(&mut harness, "editor-close");

    assert!(!is_open(&harness));
    assert_eq!(stored(&harness, &id).content, "SELECT 1;\nSELECT 2;");
    let snippet = harness
        .get_by(|node| node.author_id() == Some("note-card-snippet"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert!(snippet.contains("SELECT 2;"), "{snippet}");
}

#[test]
fn a_title_is_written_when_its_field_is_left_and_nothing_else_moves() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);
    let before = stored(&harness, &id).updated_at;

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-body");
    click_id(&mut harness, "editor-title");
    assert_eq!(stored(&harness, &id).updated_at, before, "nothing moved");
    harness
        .get_by(|node| node.author_id() == Some("editor-title"))
        .type_text(" prod");
    settle(&mut harness);
    click_id(&mut harness, "editor-body");

    assert_eq!(stored(&harness, &id).title, "psql prod");
    assert!(is_open(&harness));
}

#[test]
fn the_context_and_a_typed_tag_are_kept_on_close() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-source", "Ops / DB");
    type_into(&mut harness, "editor-tag-add", "postgres");
    click_id(&mut harness, "editor-close");

    let note = stored(&harness, &id);
    assert_eq!(note.source, "Ops / DB");
    assert_eq!(note.tags, ["postgres"]);
}

#[test]
fn a_new_note_is_written_once_and_only_once_worth_keeping() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "new-note");
    assert_eq!(written(&harness), 1);
    click_id(&mut harness, "editor-close");
    assert_eq!(written(&harness), 1);

    click_id(&mut harness, "new-note");
    type_into(&mut harness, "editor-title", "Rotate the certificate");
    type_into(&mut harness, "editor-body", "openssl req -new");
    click_id(&mut harness, "editor-close");

    assert_eq!(written(&harness), 2);
    assert_eq!(cards(&harness), 2);
}

#[test]
fn pinning_a_draft_makes_it_worth_keeping() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "new-note");
    click_id(&mut harness, "editor-pin");

    assert_eq!(written(&harness), 2);
    let opened = harness.state().state.editor.opened().unwrap();
    assert!(!opened.is_draft());
    assert!(opened.note().unwrap().pinned);
    click_id(&mut harness, "editor-close");
    assert_eq!(written(&harness), 2);
}

#[test]
fn pinning_a_note_shows_at_once_and_offers_the_undo() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-pin");

    assert!(stored(&harness, &id).pinned);
    let opened = harness.state().state.editor.opened().unwrap();
    assert!(opened.note().unwrap().pinned);
    harness.get_by(|node| node.author_id() == Some("undo-restore"));
}

#[test]
fn deleting_asks_twice_then_closes_and_offers_the_undo() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-delete");
    assert!(is_open(&harness));
    assert_eq!(written(&harness), 1);
    click_id(&mut harness, "editor-delete");

    assert!(!is_open(&harness));
    assert_eq!(written(&harness), 0);
    harness.get_by(|node| node.author_id() == Some("undo-restore"));
}

#[test]
fn escape_leaves_the_field_first_then_closes() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-body", "\n-- done");
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(is_open(&harness));
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);

    assert!(!is_open(&harness));
    assert_eq!(stored(&harness, &id).content, "SELECT 1;\n-- done");
}

#[test]
fn folding_the_header_leaves_its_summary_and_is_remembered() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-fold-header");

    assert!(
        harness
            .query_by(|node| node.author_id() == Some("editor-source"))
            .is_none()
    );
    harness.get_by(|node| node.author_id() == Some("editor-header-summary"));
    assert!(harness.state().state.settings.editor_header_folded);
    click_id(&mut harness, "editor-header-summary");
    harness.get_by(|node| node.author_id() == Some("editor-source"));
}

#[test]
fn converting_rewrites_the_body_and_the_undo_puts_it_back() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "steps").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    let session = harness.state().state.editor.opened().unwrap().session;
    click_id(&mut harness, "editor-kind");
    harness.get_by_label("Convert to a todo list").click();
    settle(&mut harness);

    assert_eq!(stored(&harness, &id).kind, NoteKind::Checklist);
    let opened = harness.state().state.editor.opened().unwrap();
    assert_ne!(opened.session, session, "the fields start again");
    click_id(&mut harness, "undo-restore");
    assert_eq!(stored(&harness, &id).kind, NoteKind::Snippet);
    assert_eq!(stored(&harness, &id).content, "SELECT 1;");
}

#[test]
fn a_deadline_picked_makes_the_note_due_and_clearing_it_makes_it_permanent() {
    use engine::notes::model::NoteLifecycle;
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-deadline");
    harness.key_press(egui::Key::ArrowRight);
    harness.key_press(egui::Key::ArrowDown);
    harness.key_press(egui::Key::Enter);
    settle(&mut harness);

    let NoteLifecycle::Expires { at } = stored(&harness, &id).lifecycle else {
        panic!("the note has no deadline");
    };
    let today = chrono::Local::now().date_naive();
    assert_eq!(
        at.with_timezone(&chrono::Local).date_naive(),
        today + chrono::Days::new(8)
    );
    harness.get_by(|node| node.author_id() == Some("editor-lifecycle"));

    click_id(&mut harness, "editor-deadline");
    click_id(&mut harness, "date-clear");
    assert_eq!(stored(&harness, &id).lifecycle, NoteLifecycle::Permanent);
}

#[test]
fn a_day_of_the_month_is_picked_by_the_pointer_and_delete_clears_it() {
    use chrono::Datelike;
    use engine::notes::model::NoteLifecycle;
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql").id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-deadline");
    click_id(&mut harness, "date-next");
    click_id(&mut harness, "date-next");
    click_id(&mut harness, "date-previous");
    let day = chrono::Local::now().date_naive().with_day(1).unwrap() + chrono::Months::new(1);
    let label = i18n::format::date_long(day, i18n::Language::En);
    harness
        .get_by(|node| {
            node.author_id() == Some("date-day") && node.label().as_deref() == Some(&label)
        })
        .click();
    settle(&mut harness);
    let NoteLifecycle::Expires { at } = stored(&harness, &id).lifecycle else {
        panic!("the note has no deadline");
    };
    assert_eq!(at.with_timezone(&chrono::Local).date_naive(), day);

    harness
        .get_by(|node| node.author_id() == Some("editor-deadline"))
        .focus();
    harness.key_press(egui::Key::Delete);
    settle(&mut harness);
    assert_eq!(stored(&harness, &id).lifecycle, NoteLifecycle::Permanent);
}

#[test]
fn a_note_is_filed_moved_and_prioritised_from_its_header() {
    use crate::notes::bench::{folder, space};
    use engine::notes::priority::Priority;
    let (backend, profile) = testing::unlocked();
    let note = testing::note(&backend, "psql");
    let home = crate::notes::bench::spaces_of_backend(&backend)
        .into_iter()
        .next()
        .unwrap();
    folder(&backend, &home, None, "Databases");
    space(&backend, "Archive");
    let id = note.id.clone();
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-folder");
    menu_entry(&mut harness, "Databases");
    settle(&mut harness);
    assert!(stored(&harness, &id).folder_id.is_some());

    click_id(&mut harness, "editor-priority");
    harness.get_by_label("High").click();
    settle(&mut harness);
    assert_eq!(stored(&harness, &id).priority, Priority::High);

    click_id(&mut harness, "editor-space");
    menu_entry(&mut harness, "Archive");
    settle(&mut harness);
    let moved = stored(&harness, &id);
    assert_ne!(moved.space_id, home.id);
    assert!(moved.folder_id.is_none(), "a move leaves the folder");
}

#[test]
fn a_draft_filed_prioritised_and_moved_is_written_with_all_of_it() {
    use crate::notes::bench::{folder, space};
    use engine::notes::priority::Priority;
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let home = crate::notes::bench::spaces_of_backend(&backend)
        .into_iter()
        .next()
        .unwrap();
    folder(&backend, &home, None, "Databases");
    space(&backend, "Archive");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "new-note");
    // The first space, whichever it is: then into Home, its folder, and out to Archive.
    click_id(&mut harness, "editor-space");
    menu_entry(&mut harness, "Home");
    click_id(&mut harness, "editor-folder");
    menu_entry(&mut harness, "Databases");
    let draft = harness.state().state.editor.opened().unwrap();
    assert!(draft.is_draft());
    assert!(draft.note().unwrap().folder_id.is_some());
    click_id(&mut harness, "editor-space");
    menu_entry(&mut harness, "Archive");
    settle(&mut harness);
    assert!(harness.state().state.editor.opened().unwrap().is_draft());
    click_id(&mut harness, "editor-priority");
    harness.get_by_label("Urgent").click();
    settle(&mut harness);

    let opened = harness.state().state.editor.opened().unwrap();
    let note = opened.note().unwrap();
    assert!(!opened.is_draft());
    assert_eq!(note.priority, Priority::Urgent);
    assert_ne!(note.space_id, home.id);
}

#[test]
fn a_draft_converted_only_changes_kind_and_a_draft_deleted_is_let_go() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "new-note");
    click_id(&mut harness, "editor-kind");
    harness.get_by_label("Convert to a todo list").click();
    settle(&mut harness);
    let opened = harness.state().state.editor.opened().unwrap();
    assert!(opened.is_draft());
    assert_eq!(opened.note().unwrap().kind, NoteKind::Checklist);

    click_id(&mut harness, "editor-delete");
    click_id(&mut harness, "editor-delete");
    assert!(!is_open(&harness));
    assert_eq!(written(&harness), 1);
}

#[test]
fn copying_from_the_editor_writes_first_and_full_screen_is_remembered() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "note-card-open");
    type_into(&mut harness, "editor-body", "\nSELECT 2;");
    harness
        .get_by(|node| node.author_id() == Some("editor-copy"))
        .click();
    let mut copied = None;
    for _ in 0..60 {
        harness.step();
        harness.state().backend.settle();
        copied = copied.or_else(|| {
            harness
                .output()
                .platform_output
                .commands
                .iter()
                .find_map(|command| match command {
                    egui::OutputCommand::CopyText(text) => Some(text.clone()),
                    _ => None,
                })
        });
    }
    assert_eq!(copied.as_deref(), Some("SELECT 1;\nSELECT 2;"));

    click_id(&mut harness, "editor-fullscreen");
    assert!(harness.state().state.settings.editor_fullscreen);
    click_id(&mut harness, "editor-fullscreen");
    assert!(!harness.state().state.settings.editor_fullscreen);
}
