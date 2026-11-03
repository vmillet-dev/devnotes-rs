//! The rail over a real core, drawn with the canvas beside it as the area draws them.

use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::folders::model::{Folder, FolderColour, FolderDraft};
use engine::spaces::model::{Space, SpaceDraft};

use crate::action::Actions;
use crate::backend::Backend;
use crate::gate::IndexFill;
use crate::services::preferences::Preferences;
use crate::state::AppState;
use crate::state::spaces::ACTIVE_SPACE_KEY;
use crate::testing;

struct Bench {
    backend: Backend,
    state: AppState,
    _profile: tempfile::TempDir,
}

fn area(backend: Backend, profile: tempfile::TempDir) -> Harness<'static, Bench> {
    let mut state = AppState::new(Preferences::default(), true);
    state.language = i18n::Language::En;
    let mut harness = testing::harness()
        .with_size(vec2(1_100.0, 700.0))
        .build_ui_state(
            |ui, bench: &mut Bench| {
                crate::theme::motion::set_reduced(ui.ctx(), true);
                let ctx = ui.ctx().clone();
                bench
                    .state
                    .update_library(&ctx, &bench.backend, IndexFill::Ready);
                let mut actions = Actions::default();
                let mut view = std::mem::take(&mut bench.state.notes_view);
                crate::notes::show(ui, &bench.state, &mut view, &mut actions);
                bench.state.notes_view = view;
                for action in actions.drain().collect::<Vec<_>>() {
                    bench.state.apply(action, &ctx, &bench.backend);
                }
            },
            Bench {
                backend,
                state,
                _profile: profile,
            },
        );
    settle(&mut harness);
    harness
}

fn settle(harness: &mut Harness<'_, Bench>) {
    testing::settle(harness, |bench| &bench.backend);
}

fn space(backend: &Backend, name: &str) -> Space {
    let draft = SpaceDraft { name: name.into() };
    engine::db::run_here(backend.engine(), move |engine| {
        engine::spaces::create_space(engine, draft)
    })
    .unwrap()
}

fn folder(backend: &Backend, space: &Space, parent: Option<&Folder>, name: &str) -> Folder {
    let draft = FolderDraft {
        space_id: space.id.clone(),
        name: name.into(),
        parent_id: parent.map(|parent| parent.id.clone()),
    };
    engine::db::run_here(backend.engine(), move |engine| {
        engine::folders::create_folder(engine, draft)
    })
    .unwrap()
}

/// The names of the rows wearing `author_id`, top to bottom.
fn names(harness: &Harness<'_, Bench>, author_id: &str) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

fn click(harness: &mut Harness<'_, Bench>, author_id: &str, name: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id) && node.label().as_deref() == Some(name))
        .click();
    settle(harness);
}

fn click_id(harness: &mut Harness<'_, Bench>, author_id: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
    settle(harness);
}

/// Into the field that has the focus, then Enter.
fn type_and_enter(harness: &mut Harness<'_, Bench>, author_id: &str, words: &str) {
    let field = harness.get_by(|node| node.author_id() == Some(author_id));
    field.type_text(words);
    settle(harness);
    harness.key_press(egui::Key::Enter);
    settle(harness);
}

fn cards(harness: &Harness<'_, Bench>) -> usize {
    harness
        .query_all_by(|node| node.author_id() == Some("note-card"))
        .count()
}

fn spaces_of(harness: &Harness<'_, Bench>) -> Vec<Space> {
    engine::db::run_here(
        harness.state().backend.engine(),
        engine::spaces::list_spaces,
    )
    .unwrap()
}

fn remembered(harness: &Harness<'_, Bench>, key: &str) -> Option<String> {
    let directory = engine::db::run_here(
        harness.state().backend.engine(),
        engine::libraries::open_library_directory,
    )
    .unwrap();
    Preferences::read(&directory.join(engine::layout::PREFERENCES))
        .get(key)
        .map(str::to_owned)
}

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
