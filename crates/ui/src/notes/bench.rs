//! An area over a real core, drawn as the window draws it — the notes' rail, header and
//! canvas, or the HTTP area — the actions applied at the end of each frame.

use egui::vec2;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use engine::folders::model::{Folder, FolderDraft};
use engine::spaces::model::{Space, SpaceDraft};

use crate::action::Actions;
use crate::backend::Backend;
use crate::gate::IndexFill;
use crate::services::preferences::Preferences;
use crate::state::{AppState, Area};
use crate::testing;

pub(crate) struct Bench {
    pub(crate) backend: Backend,
    pub(crate) state: AppState,
    _profile: tempfile::TempDir,
}

pub(crate) fn area(backend: Backend, profile: tempfile::TempDir) -> Harness<'static, Bench> {
    area_of(Area::Notes, backend, profile)
}

pub(crate) fn area_of(
    shown: Area,
    backend: Backend,
    profile: tempfile::TempDir,
) -> Harness<'static, Bench> {
    let mut state = AppState::new(Preferences::default(), true);
    state.language = i18n::Language::En;
    state.settings.area = shown;
    // Unlocked already: the gate learns it as the window's does, and stays open.
    state.gate.ask(&backend);
    let mut harness = testing::harness()
        .with_size(vec2(1_100.0, 700.0))
        .build_ui_state(
            |ui, bench: &mut Bench| {
                crate::theme::motion::set_reduced(ui.ctx(), true);
                let ctx = ui.ctx().clone();
                let state = &mut bench.state;
                state
                    .gate
                    .update(&bench.backend, &mut state.status, &mut state.errors);
                bench
                    .state
                    .update_library(&ctx, &bench.backend, IndexFill::Ready);
                let mut actions = Actions::default();
                if bench.state.settings.area == Area::Http {
                    let mut view = std::mem::take(&mut bench.state.http_view);
                    crate::http::show(ui, &bench.state, &mut view, &mut actions);
                    crate::http::leave::show(ui, &bench.state, &mut view, &mut actions);
                    bench.state.http_view = view;
                    if let Some(action) = crate::keys::dispatch(
                        &ctx,
                        &[&crate::keys::HTTP, &crate::keys::APPLICATION],
                    ) {
                        actions.push(action);
                    }
                    for action in actions.drain().collect::<Vec<_>>() {
                        bench.state.apply(action, &ctx, &bench.backend);
                    }
                    return;
                }
                if bench.state.settings.area == Area::Tools {
                    let mut view = std::mem::take(&mut bench.state.tools_view);
                    crate::tools::show(ui, &bench.state, &mut view, &mut actions);
                    bench.state.tools_view = view;
                    if let Some(action) = crate::keys::dispatch(
                        &ctx,
                        &[&crate::keys::TOOLS, &crate::keys::APPLICATION],
                    ) {
                        actions.push(action);
                    }
                    for action in actions.drain().collect::<Vec<_>>() {
                        bench.state.apply(action, &ctx, &bench.backend);
                    }
                    return;
                }
                let mut view = std::mem::take(&mut bench.state.notes_view);
                crate::notes::show(ui, &bench.state, &mut view, &mut actions);
                let mut palette = std::mem::take(&mut bench.state.palette_view);
                crate::palette::show(&ctx, &bench.state, &mut palette, &mut actions);
                bench.state.palette_view = palette;
                crate::notes::fill_form(ui, &bench.state, &mut view, &mut actions);
                bench.state.notes_view = view;
                // As the window reads them.
                let levels: &[&[crate::keys::Binding]] = if bench.state.palette.is_open() {
                    &[&crate::keys::APPLICATION]
                } else if bench.state.modal_open() {
                    &[&crate::keys::NOTES, &crate::keys::APPLICATION]
                } else {
                    &[
                        &crate::keys::NOTES,
                        &crate::notes::canvas::shortcuts::CANVAS,
                        &crate::keys::APPLICATION,
                    ]
                };
                if let Some(action) = crate::keys::dispatch(&ctx, levels) {
                    actions.push(action);
                }
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

pub(crate) fn settle(harness: &mut Harness<'_, Bench>) {
    testing::settle(harness, |bench| &bench.backend);
}

pub(crate) fn space(backend: &Backend, name: &str) -> Space {
    let draft = SpaceDraft { name: name.into() };
    engine::db::run_here(backend.engine(), move |engine| {
        engine::spaces::create_space(engine, draft)
    })
    .unwrap()
}

pub(crate) fn folder(
    backend: &Backend,
    space: &Space,
    parent: Option<&Folder>,
    name: &str,
) -> Folder {
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
pub(crate) fn names(harness: &Harness<'_, Bench>, author_id: &str) -> Vec<String> {
    harness
        .query_all_by(|node| node.author_id() == Some(author_id))
        .map(|node| node.accesskit_node().label().unwrap_or_default())
        .collect()
}

pub(crate) fn click(harness: &mut Harness<'_, Bench>, author_id: &str, name: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id) && node.label().as_deref() == Some(name))
        .click();
    settle(harness);
}

pub(crate) fn click_id(harness: &mut Harness<'_, Bench>, author_id: &str) {
    harness
        .get_by(|node| node.author_id() == Some(author_id))
        .click();
    settle(harness);
}

/// Into the field that has the focus, then Enter.
pub(crate) fn type_and_enter(harness: &mut Harness<'_, Bench>, author_id: &str, words: &str) {
    let field = harness.get_by(|node| node.author_id() == Some(author_id));
    field.type_text(words);
    settle(harness);
    harness.key_press(egui::Key::Enter);
    settle(harness);
}

pub(crate) fn cards(harness: &Harness<'_, Bench>) -> usize {
    harness
        .query_all_by(|node| node.author_id() == Some("note-card"))
        .count()
}

pub(crate) fn spaces_of_backend(backend: &Backend) -> Vec<Space> {
    engine::db::run_here(backend.engine(), engine::spaces::list_spaces).unwrap()
}

pub(crate) fn spaces_of(harness: &Harness<'_, Bench>) -> Vec<Space> {
    engine::db::run_here(
        harness.state().backend.engine(),
        engine::spaces::list_spaces,
    )
    .unwrap()
}

pub(crate) fn remembered(harness: &Harness<'_, Bench>, key: &str) -> Option<String> {
    let directory = engine::db::run_here(
        harness.state().backend.engine(),
        engine::libraries::open_library_directory,
    )
    .unwrap();
    Preferences::read(&directory.join(engine::layout::PREFERENCES))
        .get(key)
        .map(str::to_owned)
}

/// Frames until `holds`, the core settled after each: what waits on time, a debounce, needs
/// the frames to pass.
pub(crate) fn until(
    harness: &mut Harness<'_, Bench>,
    what: &str,
    holds: impl Fn(&Harness<'_, Bench>) -> bool,
) {
    for _ in 0..120 {
        settle(harness);
        if holds(harness) {
            return;
        }
        harness.step();
    }
    panic!("{what}: still not true");
}
