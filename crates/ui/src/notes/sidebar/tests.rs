//! The rail over a real core, drawn with the canvas beside it as the area draws them.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::folders::model::FolderColour;

use crate::notes::bench::{
    Bench, area, cards, click, click_id, folder, names, remembered, settle, space, spaces_of,
    type_and_enter,
};
use crate::state::spaces::ACTIVE_SPACE_KEY;
use crate::testing;

#[test]
fn spaces_and_their_folders_are_a_tree() {
    let (backend, profile) = testing::unlocked();
    let home = space(&backend, "Home");
    space(&backend, "Work");
    let runbooks = folder(&backend, &home, None, "Runbooks");
    folder(&backend, &home, Some(&runbooks), "api");
    folder(&backend, &home, None, "Archive");

    let mut harness = area(backend, profile);

    assert_eq!(names(&harness, "space-option"), ["Home", "Work"]);
    assert_eq!(
        names(&harness, "folder-option"),
        ["Archive", "Runbooks", "api"]
    );
    harness.get_by(|node| node.author_id() == Some("space-option-all"));

    click(
        &mut harness,
        "folder-twisty",
        "Collapse the folder Runbooks",
    );
    assert_eq!(names(&harness, "folder-option"), ["Archive", "Runbooks"]);
    click(&mut harness, "space-twisty", "Collapse space Home");
    assert!(names(&harness, "folder-option").is_empty());
}

#[test]
fn a_space_chosen_narrows_the_canvas_and_is_remembered() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "in home");
    let work = space(&backend, "Work");
    let draft = testing::snippet(work.id.clone(), "in work".into());
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
    let mut harness = area(backend, profile);
    assert_eq!(cards(&harness), 2);

    click(&mut harness, "space-option", "Work");

    assert_eq!(cards(&harness), 1);
    assert_eq!(
        remembered(&harness, ACTIVE_SPACE_KEY).as_deref(),
        Some(work.id.as_str())
    );
    click_id(&mut harness, "space-option-all");
    assert_eq!(cards(&harness), 2);
}

#[test]
fn a_folder_chosen_shows_its_notes_in_its_space() {
    let (backend, profile) = testing::unlocked();
    let filed = testing::note(&backend, "filed");
    testing::note(&backend, "loose");
    let spaces = engine::db::run_here(backend.engine(), engine::spaces::list_spaces).unwrap();
    let runbooks = folder(&backend, &spaces[0], None, "Runbooks");
    let (id, into) = (filed.id.clone(), runbooks.id.clone());
    engine::db::run_here(backend.engine(), move |engine| {
        engine::folders::file_notes(engine, vec![id], Some(into))
    })
    .unwrap();
    let mut harness = area(backend, profile);

    click(&mut harness, "folder-option", "Runbooks");

    assert_eq!(cards(&harness), 1);
    assert_eq!(
        harness.state().state.spaces.active_space(),
        Some(spaces[0].id.as_str())
    );
}

#[test]
fn a_space_is_created_from_the_head_and_shown() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "in home");
    let mut harness = area(backend, profile);

    click_id(&mut harness, "space-create-open");
    type_and_enter(&mut harness, "space-create-input", "Reading");

    assert_eq!(names(&harness, "space-option"), ["Home", "Reading"]);
    let reading = spaces_of(&harness)
        .into_iter()
        .find(|space| space.name == "Reading")
        .unwrap();
    assert_eq!(
        harness.state().state.spaces.active_space(),
        Some(reading.id.as_str())
    );
    assert_eq!(cards(&harness), 0);
}

#[test]
fn a_space_is_renamed_when_its_field_is_left_and_pinned_from_its_panel() {
    let (backend, profile) = testing::unlocked();
    space(&backend, "Home");
    let mut harness = area(backend, profile);

    click(&mut harness, "space-edit", "Options for space Home");
    let field = harness.get_by(|node| node.author_id() == Some("space-rename-input"));
    field.type_text(" base");
    settle(&mut harness);
    click_id(&mut harness, "space-pin");

    let spaces = spaces_of(&harness);
    assert_eq!(spaces[0].name, "Home base");
    assert!(spaces[0].pinned);
    assert_eq!(names(&harness, "space-option"), ["Home base"]);
}

#[test]
fn the_only_space_cannot_be_deleted() {
    let (backend, profile) = testing::unlocked();
    space(&backend, "Home");
    let mut harness = area(backend, profile);

    click(&mut harness, "space-edit", "Options for space Home");

    harness.get_by(|node| node.author_id() == Some("space-delete-blocked"));
    assert!(
        harness
            .query_by(|node| node.author_id() == Some("space-delete"))
            .is_none()
    );
}

#[test]
fn deleting_a_space_says_where_its_notes_go_then_moves_them_there() {
    let (backend, profile) = testing::unlocked();
    let home = space(&backend, "Home");
    let reading = space(&backend, "Reading");
    folder(&backend, &reading, None, "Read");
    let draft = testing::snippet(reading.id.clone(), "an article".into());
    engine::db::run_here(backend.engine(), move |engine| {
        engine::notes::create_note(engine, draft)
    })
    .unwrap();
    let mut harness = area(backend, profile);

    click(&mut harness, "space-edit", "Options for space Reading");
    let said = harness
        .get_by(|node| node.author_id() == Some("space-delete-consequence"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert!(said.contains("“Home”"), "{said}");
    assert!(said.contains("folder goes with it"), "{said}");
    click_id(&mut harness, "space-delete");
    assert_eq!(spaces_of(&harness).len(), 2);
    click_id(&mut harness, "space-delete");

    assert_eq!(names(&harness, "space-option"), ["Home"]);
    assert_eq!(
        harness.state().state.spaces.active_space(),
        Some(home.id.as_str())
    );
    assert_eq!(cards(&harness), 1);
}

#[test]
fn a_folder_is_made_recoloured_nested_and_deleted_from_the_rail() {
    let (backend, profile) = testing::unlocked();
    let home = space(&backend, "Home");
    let mut harness = area(backend, profile);
    click(&mut harness, "space-option", "Home");

    click_id(&mut harness, "folder-create-open");
    type_and_enter(&mut harness, "folder-create-input", "Runbooks");
    assert_eq!(names(&harness, "folder-option"), ["Runbooks"]);

    click(&mut harness, "folder-edit", "Options for folder Runbooks");
    click(&mut harness, "folder-colour", "Green");
    click_id(&mut harness, "folder-subfolder-open");
    type_and_enter(&mut harness, "folder-subfolder-input", "api");
    assert_eq!(names(&harness, "folder-option"), ["Runbooks", "api"]);

    let folders = |harness: &Harness<'_, Bench>| {
        let id = home.id.clone();
        engine::db::run_here(harness.state().backend.engine(), move |engine| {
            engine::folders::list_folders(engine, Some(id))
        })
        .unwrap()
    };
    let runbooks = folders(&harness)
        .into_iter()
        .find(|folder| folder.name == "Runbooks")
        .unwrap();
    assert_eq!(runbooks.colour, FolderColour::Green);

    click(&mut harness, "folder-edit", "Options for folder Runbooks");
    click_id(&mut harness, "folder-delete");
    click_id(&mut harness, "folder-delete");

    let left = folders(&harness);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].parent_id, None);
}

/// A press on `from`, the pointer moved in steps, a release on `to`.
fn drag(harness: &mut Harness<'_, Bench>, from: egui::Pos2, to: egui::Pos2) {
    let press = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.event(egui::Event::PointerMoved(from));
    harness.step();
    harness.event(press(from, true));
    harness.step();
    for step in [0.25, 0.5, 0.75, 1.0] {
        harness.event(egui::Event::PointerMoved(from + (to - from) * step));
        harness.step();
    }
    harness.event(press(to, false));
    settle(harness);
    harness.event(egui::Event::PointerGone);
    settle(harness);
}

fn row(harness: &Harness<'_, Bench>, author_id: &str, name: &str) -> egui::Rect {
    harness
        .get_by(|node| node.author_id() == Some(author_id) && node.label().as_deref() == Some(name))
        .rect()
}

fn parent_of(harness: &Harness<'_, Bench>, name: &str) -> Option<String> {
    let parent = harness
        .state()
        .state
        .folders
        .list()
        .iter()
        .find(|folder| folder.name == name)
        .and_then(|folder| folder.parent_id.clone())?;
    harness
        .state()
        .state
        .folders
        .find(&parent)
        .map(|folder| folder.name.clone())
}

#[test]
fn a_folder_dragged_onto_another_goes_inside_it_and_the_undo_bar_puts_it_back() {
    let (backend, profile) = testing::unlocked();
    let home = space(&backend, "Home");
    folder(&backend, &home, None, "Bases");
    folder(&backend, &home, None, "Index");
    let mut harness = area(backend, profile);
    click(&mut harness, "space-option", "Home");

    let from = row(&harness, "folder-option", "Index").center();
    let to = row(&harness, "folder-option", "Bases").center();
    drag(&mut harness, from, to);
    assert_eq!(parent_of(&harness, "Index").as_deref(), Some("Bases"));
    let said = harness
        .get_by(|node| node.author_id() == Some("undo-text"))
        .accesskit_node()
        .value()
        .unwrap_or_default();
    assert!(said.contains("Index") && said.contains("Bases"), "{said}");

    // Onto what it holds is refused, and said.
    let from = row(&harness, "folder-option", "Bases").center();
    let to = row(&harness, "folder-option", "Index").center();
    drag(&mut harness, from, to);
    assert_eq!(parent_of(&harness, "Bases"), None);
    let said = harness
        .state()
        .state
        .status
        .latest()
        .map(|(_, message)| message.key);
    assert_eq!(said, Some(i18n::keys::sidebar::moved::refused::THERE));

    click_id(&mut harness, "undo-restore");
    assert_eq!(parent_of(&harness, "Index"), None);
}

#[test]
fn alt_and_the_arrows_move_a_folder_in_and_out_saying_what_they_refuse() {
    let (backend, profile) = testing::unlocked();
    let home = space(&backend, "Home");
    folder(&backend, &home, None, "Bases");
    folder(&backend, &home, None, "Index");
    let mut harness = area(backend, profile);
    click(&mut harness, "space-option", "Home");
    let press = |harness: &mut Harness<'_, Bench>, name: &str, key| {
        harness
            .get_by(|node| {
                node.author_id() == Some("folder-option") && node.label().as_deref() == Some(name)
            })
            .focus();
        harness.step();
        harness.key_press_modifiers(egui::Modifiers::ALT, key);
        settle(harness);
    };

    press(&mut harness, "Index", egui::Key::ArrowRight);
    assert_eq!(parent_of(&harness, "Index").as_deref(), Some("Bases"));
    press(&mut harness, "Index", egui::Key::ArrowLeft);
    assert_eq!(parent_of(&harness, "Index"), None);

    press(&mut harness, "Bases", egui::Key::ArrowLeft);
    let said = |harness: &Harness<'_, Bench>| {
        harness
            .state()
            .state
            .status
            .latest()
            .map(|(_, message)| message.key)
    };
    assert_eq!(
        said(&harness),
        Some(i18n::keys::sidebar::moved::refused::AT_TOP)
    );
    press(&mut harness, "Bases", egui::Key::ArrowRight);
    assert_eq!(
        said(&harness),
        Some(i18n::keys::sidebar::moved::refused::NO_FOLDER_ABOVE)
    );
}
