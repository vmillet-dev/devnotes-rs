use egui_kittest::kittest::{NodeT, Queryable};
use engine::notes::kind::NoteKind;
use engine::notes::model::DisplayNote;

use crate::notes::bench::{Bench, area, cards, click_id, settle};
use crate::testing;

fn stored(harness: &egui_kittest::Harness<'_, Bench>, id: &str) -> DisplayNote {
    let id = id.to_owned();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, &id)
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

#[test]
fn expanded_the_note_leaves_the_header_and_its_sidebar_toggle_in_reach() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = area(backend, profile);
    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-fullscreen");

    let toggle = harness
        .get_by(|node| node.author_id() == Some("library-rail-toggle"))
        .rect();
    let expand = harness
        .get_by(|node| node.author_id() == Some("editor-fullscreen"))
        .rect();
    assert!(
        toggle.bottom() < expand.top(),
        "{toggle:?} above {expand:?}"
    );
    click_id(&mut harness, "library-rail-toggle");

    assert!(!harness.state().state.settings.rail_shown);
    assert!(is_open(&harness) && harness.state().state.settings.editor_fullscreen);
}

/// Wide enough for the panel beside two columns of cards.
fn wide(
    mut harness: egui_kittest::Harness<'static, Bench>,
) -> egui_kittest::Harness<'static, Bench> {
    harness.set_size(egui::vec2(1_440.0, 800.0));
    settle(&mut harness);
    harness
}

fn rect_of(harness: &egui_kittest::Harness<'_, Bench>, id: &str) -> egui::Rect {
    harness.get_by(|node| node.author_id() == Some(id)).rect()
}

fn title_of_open(harness: &egui_kittest::Harness<'_, Bench>) -> String {
    let opened = harness.state().state.editor.opened().unwrap();
    opened.note().unwrap().title.clone()
}

/// Closed, beside the cards, expanded: the header keeps its place, and the note is under it.
#[test]
fn the_header_spans_the_area_whether_the_note_is_beside_the_cards_or_expanded() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql");
    let mut harness = wide(area(backend, profile));
    let controls = |harness: &egui_kittest::Harness<'_, Bench>| {
        [
            "new-note",
            "search-input",
            "facets-toggle",
            "library-rail-toggle",
        ]
        .map(|id| rect_of(harness, id))
    };
    let closed = controls(&harness);

    click_id(&mut harness, "note-card-open");
    assert_eq!(controls(&harness), closed, "beside the cards");
    let edge = rect_of(&harness, "editor-panel-edge");
    let lowest = closed.iter().map(egui::Rect::bottom).fold(0.0, f32::max);
    assert!(edge.top() >= lowest, "{edge:?} under {lowest}");
    let close = rect_of(&harness, "editor-close").top();
    assert!(close >= edge.top());

    click_id(&mut harness, "editor-fullscreen");
    assert_eq!(controls(&harness), closed, "expanded");
}

/// The cards stay on screen beside the note open; expanded, the note takes the area, and the
/// way back brings the cards again.
#[test]
fn a_note_opens_beside_the_cards_and_expands_over_them() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = wide(area(backend, profile));

    click_id(&mut harness, "note-card-open");

    assert!(is_open(&harness));
    assert_eq!(cards(&harness), 1, "the card is still drawn");
    assert!(rect_of(&harness, "note-card").right() <= rect_of(&harness, "editor-title").left());

    click_id(&mut harness, "editor-fullscreen");
    assert_eq!(cards(&harness), 0);
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("editor-breadcrumb"))
            .is_some()
    );

    click_id(&mut harness, "editor-back-to-cards");
    assert_eq!(cards(&harness), 1);
    assert!(is_open(&harness));
}

/// From one open note to another: on every frame until the second is read, the panel keeps its
/// place and the cards beside it their width.
#[test]
fn moving_to_another_note_keeps_the_panel_in_place() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "first");
    testing::note(&backend, "second");
    let mut harness = wide(area(backend, profile));
    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some("first")
        })
        .click();
    settle(&mut harness);
    let header = rect_of(&harness, "new-note");

    harness
        .get_by(|node| {
            node.author_id() == Some("note-card-open") && node.label().as_deref() == Some("second")
        })
        .click();
    for frame in 0..500 {
        harness.step();
        let now = rect_of(&harness, "new-note");
        assert_eq!(now, header, "the cards' width moved at frame {frame}");
        let read = harness
            .state()
            .state
            .editor
            .opened()
            .and_then(|open| open.note());
        if read.is_some_and(|note| note.title == "second") {
            return;
        }
    }
    settle(&mut harness);
    assert_eq!(title_of_open(&harness), "second");
    assert_eq!(rect_of(&harness, "new-note"), header);
}

/// A card clicked while a note is open: what was typed in the first is written on the way.
#[test]
fn another_card_opens_once_the_note_open_wrote_its_title() {
    let (backend, profile) = testing::unlocked();
    let first = testing::note(&backend, "first").id.clone();
    testing::note(&backend, "second");
    let mut harness = wide(area(backend, profile));
    crate::notes::bench::click(&mut harness, "note-card-open", "first");
    click_id(&mut harness, "editor-title");
    harness.key_press(egui::Key::End);
    harness
        .get_by(|node| node.author_id() == Some("editor-title"))
        .type_text(" edited");
    settle(&mut harness);

    crate::notes::bench::click(&mut harness, "note-card-open", "second");

    assert_eq!(title_of_open(&harness), "second");
    assert_eq!(stored(&harness, &first).title, "first edited");
}

fn title_has_focus(harness: &egui_kittest::Harness<'_, Bench>) -> bool {
    harness
        .get_by(|node| node.author_id() == Some("editor-title"))
        .accesskit_node()
        .is_focused()
}

#[test]
fn a_note_opened_from_its_card_leaves_the_keys_to_j_and_k() {
    let (backend, profile) = testing::unlocked();
    for title in ["one", "two"] {
        testing::note(&backend, title);
    }
    let mut harness = wide(area(backend, profile));
    let order = harness.state().state.on_screen();

    crate::notes::bench::click(&mut harness, "note-card-open", "two");
    let first = title_of_open(&harness);
    assert!(!title_has_focus(&harness));
    let step = if harness.state().state.editor.open_id() == Some(order[0].as_str()) {
        egui::Key::J
    } else {
        egui::Key::K
    };
    harness.key_press(step);
    settle(&mut harness);
    assert_ne!(title_of_open(&harness), first);
    assert!(!title_has_focus(&harness));

    harness.key_press(egui::Key::F2);
    settle(&mut harness);
    assert!(title_has_focus(&harness));
}

#[test]
fn a_new_note_gives_its_title_the_focus() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "one");
    let mut harness = wide(area(backend, profile));

    click_id(&mut harness, "new-note");

    assert!(title_has_focus(&harness));
}

/// J and K walk the cards as they are shown, once no field has the focus; expanded, the
/// position says where the note sits; Escape leaves full screen, then closes.
#[test]
fn j_and_k_step_through_the_cards_and_escape_steps_back() {
    let (backend, profile) = testing::unlocked();
    for title in ["one", "two", "three"] {
        testing::note(&backend, title);
    }
    let mut harness = area(backend, profile);
    let order = harness.state().state.on_screen();
    let first = order[0].clone();
    harness.state_mut().state.editor.open(first);
    settle(&mut harness);
    assert!(is_open(&harness));

    harness.key_press(egui::Key::J);
    settle(&mut harness);
    assert_eq!(
        harness.state().state.editor.open_id(),
        Some(order[1].as_str())
    );
    harness.key_press(egui::Key::K);
    settle(&mut harness);
    assert_eq!(
        harness.state().state.editor.open_id(),
        Some(order[0].as_str())
    );

    click_id(&mut harness, "editor-fullscreen");
    let position = harness
        .get_by(|node| node.author_id() == Some("editor-position"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert_eq!(position, "1 / 3");
    click_id(&mut harness, "editor-next");
    assert_eq!(
        harness.state().state.editor.open_id(),
        Some(order[1].as_str())
    );

    // Stepped to, the note leaves the focus where it was: one Escape leaves full screen.
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(is_open(&harness) && !harness.state().state.settings.editor_fullscreen);
    harness.key_press(egui::Key::Escape);
    settle(&mut harness);
    assert!(!is_open(&harness));
}

/// A note holding all a header can, its files included, open in a window `width` wide.
fn full_header(width: f32) -> egui_kittest::Harness<'static, Bench> {
    use engine::notes::model::{NoteLifecycle, NotePatch};
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "Rotation").id.clone();
    let mut harness = area(backend, profile);
    let note = id.clone();
    engine::db::run_here(harness.state().backend.engine(), move |engine| {
        let space_id = engine::notes::get_note(engine, &note)?.space_id.clone();
        let folder = engine::folders::create_folder(
            engine,
            &engine::folders::model::FolderDraft {
                space_id,
                name: "Certificates and their yearly rotation".to_owned(),
                parent_id: None,
            },
        )?;
        let lines = engine::folders::board::PreviewLines::default();
        let ids = std::slice::from_ref(&note);
        engine::folders::file_notes(engine, ids, Some(&folder.id), lines)?;
        let at = chrono::DateTime::parse_from_rfc3339("2026-12-24T22:00:00Z")
            .unwrap()
            .to_utc();
        let long = "a-tag-longer-than-a-whole-line-of-the-panel-at-its-narrowest-which-is-cut";
        let tags = ["infrastructure", "certificates", "staging", "openssl", long];
        engine::notes::update_note(
            engine,
            &note,
            &NotePatch {
                source: Some("https://wiki.example.test/runbooks/certificates".to_owned()),
                tags: Some(tags.map(str::to_owned).to_vec()),
                lifecycle: Some(NoteLifecycle::Expires { at }),
                ..NotePatch::default()
            },
        )
    })
    .unwrap();
    harness.state_mut().state.editor.open(id);
    harness.set_size(egui::vec2(width, 800.0));
    settle(&mut harness);
    let files = tempfile::tempdir().unwrap();
    for name in ["first", "second"] {
        let name = format!("the-{name}-chain-of-the-staging-certificate.pem");
        let file = files.path().join(name);
        std::fs::write(&file, b"chain").unwrap();
        harness.state_mut().state.dialog = std::sync::Arc::new(platform::dialog::Fixed(file));
        click_id(&mut harness, "attachment-add");
    }
    harness
}

#[test]
fn everything_of_the_header_is_inside_the_panel_at_its_narrowest() {
    // Beside the cards, then over them in the narrowest window.
    for width in [1_220.0, 640.0] {
        let harness = full_header(width);
        let count = |id: &str| {
            harness
                .query_all_by(|node| node.author_id() == Some(id))
                .count()
        };
        assert_eq!(count("editor-tag"), 5);
        assert_eq!(count("attachment-remove"), 2);
        for id in ["editor-lifecycle", "editor-deadline", "editor-folder"] {
            assert_eq!(count(id), 1, "{id}");
        }
        let left = rect_of(&harness, "editor-source").left();
        for node in harness.query_all_by(|node| {
            node.author_id()
                .is_some_and(|id| id.starts_with("editor-") || id.starts_with("attachment-"))
        }) {
            let id = node
                .accesskit_node()
                .author_id()
                .unwrap_or_default()
                .to_owned();
            let rect = node.rect();
            assert!(
                rect.right() <= width && (rect.left() >= left || id == "editor-panel-edge"),
                "{id} at {rect:?} in {width}"
            );
        }
    }
}

#[test]
fn the_folded_header_stays_inside_the_panel() {
    let mut harness = full_header(1_220.0);
    harness.state_mut().state.settings.editor_header_folded = true;
    settle(&mut harness);

    for id in ["editor-header-summary", "editor-lifecycle"] {
        let rect = rect_of(&harness, id);
        assert!(rect.right() <= 1_220.0, "{id} at {rect:?}");
    }
}

/// Too narrow for both, the note takes the area beside the rail.
#[test]
fn in_a_narrow_window_the_note_covers_the_cards() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    harness.set_size(egui::vec2(760.0, 700.0));
    settle(&mut harness);

    click_id(&mut harness, "note-card-open");

    assert!(is_open(&harness));
    assert_eq!(cards(&harness), 0);
}

/// Beside the cards the toolbar keeps two lines; folding and deleting wait in ⋯, and at least
/// two columns of cards stay beside it.
#[test]
fn beside_the_cards_the_seldom_used_wait_in_the_menu() {
    let (backend, profile) = testing::unlocked();
    let id = testing::note(&backend, "psql prod").id.clone();
    testing::note(&backend, "second");
    let mut harness = wide(area(backend, profile));
    crate::notes::bench::click(&mut harness, "note-card-open", "psql prod");

    let cards: Vec<egui::Rect> = harness
        .query_all_by(|node| node.author_id() == Some("note-card"))
        .map(|node| node.rect())
        .collect();
    assert!(
        cards.len() == 2 && (cards[0].top() - cards[1].top()).abs() < 1.0,
        "{cards:?}"
    );
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("editor-delete"))
            .is_none()
    );

    click_id(&mut harness, "editor-more");
    click_id(&mut harness, "editor-delete");
    click_id(&mut harness, "editor-delete");

    assert!(!is_open(&harness));
    let gone = engine::db::run_here(harness.state().backend.engine(), move |engine| {
        engine::notes::get_note(engine, &id)
    });
    assert!(gone.is_err(), "in the trash");
}

fn columns(harness: &egui_kittest::Harness<'_, Bench>) -> usize {
    let mut lefts: Vec<f32> = harness
        .query_all_by(|node| node.author_id() == Some("note-card"))
        .map(|node| node.rect().left().round())
        .collect();
    lefts.sort_by(f32::total_cmp);
    lefts.dedup();
    lefts.len()
}

/// The edge between the cards and the panel moves: the cards take the columns they have room
/// for, and the width chosen stays.
#[test]
fn the_panels_edge_moves_and_the_cards_follow() {
    let (backend, profile) = testing::unlocked();
    for title in ["one", "two", "three", "four"] {
        testing::note(&backend, title);
    }
    let mut harness = wide(area(backend, profile));
    crate::notes::bench::click(&mut harness, "note-card-open", "one");
    assert_eq!(columns(&harness), 2);

    let edge = rect_of(&harness, "editor-panel-edge").center();
    let to = edge + egui::vec2(160.0, 0.0);
    harness.event(egui::Event::PointerMoved(edge));
    harness.run_steps(1);
    harness.event(egui::Event::PointerButton {
        pos: edge,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run_steps(1);
    for step in 1..=8_u8 {
        harness.event(egui::Event::PointerMoved(
            edge + egui::vec2(20.0 * f32::from(step), 0.0),
        ));
        harness.run_steps(1);
    }
    harness.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    settle(&mut harness);

    assert_eq!(columns(&harness), 3);
    // Moved past its narrowest, the panel stops there, and that is what is kept.
    assert_eq!(harness.state().state.settings.editor_panel_width, 460);
}

/// Expanded, the details stand in a column beside the body for every kind, or are put away.
#[test]
fn expanded_the_details_stand_beside_the_body_and_can_be_put_away() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = wide(area(backend, profile));
    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-fullscreen");

    assert!(rect_of(&harness, "editor-space").left() > rect_of(&harness, "editor-body").right());
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("editor-fold-header"))
            .is_none()
    );

    click_id(&mut harness, "editor-details");
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("editor-space"))
            .is_none()
    );
    assert!(!harness.state().state.settings.editor_details_open);
}

/// Expanded in a window too narrow for the column, the header's bands hold the details.
#[test]
fn expanded_without_room_for_the_details_the_header_holds_them() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let mut harness = area(backend, profile);
    click_id(&mut harness, "note-card-open");
    click_id(&mut harness, "editor-fullscreen");

    assert!(rect_of(&harness, "editor-space").bottom() < rect_of(&harness, "editor-body").top());
}
