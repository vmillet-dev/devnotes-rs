//! What the interface remembers between frames, apart from the drawing (plan § 3.4).

pub mod drawing_pictures;
pub mod editor;
pub mod notes;
pub mod notes_query;
mod query;
pub mod spaces;

pub use query::QueryCell;

use crate::action::{Action, AppAction, GlobalAction};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::gate::{Gate, GateView, IndexFill};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;
use crate::services::samples;
use crate::services::settings::Settings;
use crate::services::status::StatusNotifier;
use crate::services::time::{Clock, SystemClock};
use crate::theme;
use notes_query::NotesQueryState;

/// Bumped after every write the interface adopts: the queries that read notes carry it among
/// their parameters, so the list and the board ask again together. Writes are not optimistic:
/// persist, adopt the answer, bump.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Revision(u64);

impl Revision {
    pub fn bump(&mut self) {
        self.0 += 1;
    }
}

/// Every cache of a library's content — laid-out text, textures, memos, diagrams — and only
/// they: one field, emptied when another library opens (D17). Not wiped byte by byte; the
/// threat model says so.
#[derive(Default)]
pub struct LibraryCaches {
    pub thumbnails: drawing_pictures::Thumbnails,
}

/// The three areas, in the switch's order, which is also the order of their keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Area {
    #[default]
    Notes,
    Tools,
    Http,
}

/// The root of the interface's state. Each screen joins it as a pair — its state, which other
/// screens and the core see, and its view, which only its drawing sees (D13) — as it is ported.
pub struct AppState {
    /// Nothing else is drawn until it says the library is open.
    pub gate: Gate,
    pub gate_view: GateView,
    /// The application's settings, which outlive a library switch.
    pub settings: Settings,
    pub language: i18n::Language,
    /// The application's whole file, the keys this version does not know included.
    pub preferences: Preferences,
    /// The open library's file (`devnotes.notes.*`), read again each time one opens.
    pub library_preferences: Preferences,
    library_preferences_read: Option<Pending<Preferences>>,
    /// The first launch's samples, being written; nothing reads notes meanwhile.
    seeding: Option<Pending<bool>>,
    pub revision: Revision,
    pub notes_query: NotesQueryState,
    pub spaces: spaces::SpacesState,
    /// The area's view: only its drawing reads it (D13).
    pub notes_view: crate::notes::View,
    pub notes: notes::NotesState,
    pub editor: editor::EditorState,
    pub caches: LibraryCaches,
    pub status: StatusNotifier,
    pub errors: ErrorNotifier,
    pub clock: Box<dyn Clock>,
    /// The system's wish; the look adds nothing to it.
    pub reduced_motion: bool,
}

impl AppState {
    pub fn new(preferences: Preferences, reduced_motion: bool) -> Self {
        let settings = Settings::read(&preferences);
        Self {
            gate: Gate::default(),
            gate_view: GateView::default(),
            language: settings.locale.language(),
            settings,
            preferences,
            library_preferences: Preferences::default(),
            library_preferences_read: None,
            seeding: None,
            revision: Revision::default(),
            notes_query: NotesQueryState::default(),
            spaces: spaces::SpacesState::default(),
            notes_view: crate::notes::View::default(),
            notes: notes::NotesState::default(),
            editor: editor::EditorState::default(),
            caches: LibraryCaches::default(),
            status: StatusNotifier::default(),
            errors: ErrorNotifier::default(),
            clock: Box::new(SystemClock),
            reduced_motion,
        }
    }

    /// Once a frame, before drawing: the answers that arrived are adopted.
    pub fn update(&mut self, ctx: &egui::Context, backend: &Backend) {
        if self
            .gate
            .update(backend, &mut self.status, &mut self.errors)
        {
            self.reset_library_scope(ctx, backend);
        }
        self.adopt_library_preferences(backend);
        self.adopt_seeding(backend);
        if self.gate.is_open() && self.library_preferences_read.is_none() && self.seeding.is_none()
        {
            self.update_library(ctx, backend, self.gate.index);
        }
    }

    /// What reads the open library, once its preferences are in and its samples seeded.
    pub fn update_library(&mut self, ctx: &egui::Context, backend: &Backend, index: IndexFill) {
        self.notes.update(&mut self.revision, &mut self.errors);
        self.editor.update(&mut self.revision, &mut self.errors);
        self.caches.thumbnails.update(ctx, backend);
        if self
            .spaces
            .update(backend, &mut self.revision, &mut self.errors)
        {
            self.remember_place(backend);
        }
        let now = self.clock.now();
        let place = (self.spaces.active_space(), self.spaces.active_folder());
        self.notes_query
            .update(ctx, backend, now, place, (self.revision, index));
    }

    /// The space and the folder shown, into the library's preferences: the next launch opens
    /// on them.
    fn remember_place(&mut self, backend: &Backend) {
        self.spaces.remember(&mut self.library_preferences);
        drop(api::preferences::write_library(
            backend,
            self.library_preferences.clone(),
        ));
    }

    /// The open library's preferences; a library that was never seeded is seeded now.
    fn adopt_library_preferences(&mut self, backend: &Backend) {
        let Some(answer) = self
            .library_preferences_read
            .as_mut()
            .and_then(Pending::poll)
        else {
            return;
        };
        self.library_preferences_read = None;
        match answer {
            Ok(preferences) => {
                if preferences.get(samples::SEEDED_KEY).is_none() {
                    let samples = samples::samples(self.language, self.clock.now());
                    self.seeding = Some(api::notes::seed_samples(backend, samples));
                }
                self.spaces.restore(&preferences);
                self.notes_query.restore(&preferences);
                self.library_preferences = preferences;
            }
            Err(error) => self
                .errors
                .report(i18n::keys::errors::LIBRARY_ACTION_FAILED, &error),
        }
    }

    /// ⚠️ The mark is written after the samples, never before: a seeding cut short rolls back
    /// whole, and the next open finds neither the mark nor a space, and seeds again.
    fn adopt_seeding(&mut self, backend: &Backend) {
        let Some(answer) = self.seeding.as_mut().and_then(Pending::poll) else {
            return;
        };
        self.seeding = None;
        match answer {
            Ok(seeded) => {
                let mark = if seeded { "true" } else { "skipped" };
                self.library_preferences.set(samples::SEEDED_KEY, mark);
                drop(api::preferences::write_library(
                    backend,
                    self.library_preferences.clone(),
                ));
                self.revision.bump();
            }
            // Silent: the canvas says its own failures, and samples nobody asked for are not
            // worth a banner.
            Err(error) => log::warn!("The samples were not seeded: {:?}", error.code),
        }
    }

    /// At the end of the frame, in the order the actions were pushed.
    pub fn apply(&mut self, action: Action, ctx: &egui::Context, backend: &Backend) {
        match action {
            Action::App(AppAction::ShowArea(area)) => {
                self.settings.area = area;
                self.save_settings(backend);
            }
            Action::App(AppAction::SetAppearance(appearance)) => {
                self.settings.theme = appearance;
                self.apply_look(ctx);
                self.save_settings(backend);
            }
            Action::App(AppAction::DismissError) => self.errors.dismiss(),
            Action::App(AppAction::DismissStatus) => self.status.dismiss(),
            Action::App(AppAction::SetRailWidth(width)) => {
                self.settings.rail_width = width;
                self.save_settings(backend);
            }
            Action::Gate(action) => self.gate.apply(action, &mut self.gate_view, backend),
            Action::Notes(action) => self.apply_notes(action, ctx, backend),
            Action::Global(GlobalAction::Quit) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            // The capture, the new note and the palette arrive with the notes area.
            Action::Global(
                GlobalAction::Capture
                | GlobalAction::NewNote
                | GlobalAction::Palette
                | GlobalAction::PaletteHere,
            ) => {}
        }
    }

    fn apply_notes(
        &mut self,
        action: crate::notes::Action,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        match action {
            crate::notes::Action::Reload => self.revision.bump(),
            crate::notes::Action::SetItems { id, items } => {
                self.notes.set_items(backend, id, items);
            }
            crate::notes::Action::Thumbnail(stamp) => self.caches.thumbnails.ask(backend, stamp),
            crate::notes::Action::ChooseSpace(id) => {
                self.spaces.choose_space(id);
                self.remember_place(backend);
            }
            crate::notes::Action::ChooseFolder(id) => {
                self.spaces.choose_folder(&id);
                self.remember_place(backend);
            }
            crate::notes::Action::CountSpace(id) => self.spaces.count(id),
            crate::notes::Action::CreateSpace(name) => self.spaces.create_space(backend, &name),
            crate::notes::Action::RenameSpace { id, name } => {
                self.spaces.rename_space(backend, id, &name);
            }
            crate::notes::Action::PinSpace { id, pinned } => {
                self.spaces.pin_space(backend, id, pinned);
            }
            crate::notes::Action::DeleteSpace { id, refuge } => {
                self.spaces.delete_space(backend, id, refuge);
            }
            crate::notes::Action::CreateFolder {
                space_id,
                parent_id,
                name,
            } => self
                .spaces
                .create_folder(backend, space_id, parent_id, &name),
            crate::notes::Action::RenameFolder { id, name } => {
                self.spaces.rename_folder(backend, id, &name);
            }
            crate::notes::Action::RecolourFolder { id, colour } => {
                self.spaces.recolour_folder(backend, id, colour);
            }
            crate::notes::Action::DeleteFolder(id) => self.spaces.delete_folder(backend, id),
            crate::notes::Action::Search(typed) => self.notes_query.type_search(ctx, typed),
            crate::notes::Action::ClearSearch => self.notes_query.clear_search(),
            crate::notes::Action::SetFilter(filter) => self.notes_query.set_filter(filter),
            crate::notes::Action::Arrange(arrangement) => {
                self.notes_query.arrange(arrangement);
                arrangement.write(&mut self.library_preferences);
                drop(api::preferences::write_library(
                    backend,
                    self.library_preferences.clone(),
                ));
            }
            crate::notes::Action::OpenNote(id) => self.editor.open(backend, id),
            crate::notes::Action::SaveNote { id, title, body } => {
                self.editor.save(backend, id, title, body);
            }
            crate::notes::Action::CloseNote => self.editor.close(),
            crate::notes::Action::FocusSearch => {
                self.notes_view.focus_search = true;
                // Taken by the next frame, which nothing else may ask for.
                ctx.request_repaint();
            }
        }
    }

    /// The settings into the application's file, through the db queue, the other keys kept.
    fn save_settings(&mut self, backend: &Backend) {
        self.settings.write(&mut self.preferences);
        // A write that fails says so in the log; nothing on screen waits for it.
        drop(api::preferences::write_application(
            backend,
            self.preferences.clone(),
        ));
    }

    /// What reloading the page did in the reference: another library is open, so nothing of
    /// the previous one may stay — its preferences, the caches, and every widget's state egui
    /// keeps (a text field's undo history holds what was typed). The settings stay.
    pub fn reset_library_scope(&mut self, ctx: &egui::Context, backend: &Backend) {
        self.library_preferences = Preferences::default();
        self.library_preferences_read = Some(api::preferences::library(backend));
        self.seeding = None;
        self.caches = LibraryCaches::default();
        self.notes_query = NotesQueryState::default();
        self.spaces = spaces::SpacesState::default();
        self.editor = editor::EditorState::default();
        self.notes_view = crate::notes::View::default();
        self.revision.bump();
        ctx.memory_mut(|memory| memory.data.clear());
        self.apply_look(ctx);
    }

    /// The look into egui, which keeps it in the memory a library switch clears.
    pub fn apply_look(&self, ctx: &egui::Context) {
        theme::motion::set_reduced(ctx, self.reduced_motion);
        theme::apply(ctx, self.settings.theme, self.settings.density);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use crate::theme::{Appearance, Density};

    fn compact() -> Preferences {
        let mut preferences = Preferences::default();
        preferences.set("devnotes.density", "compact");
        preferences.set("devnotes.someday", "kept");
        preferences
    }

    #[test]
    fn a_library_switch_reopens_its_preferences_and_keeps_the_settings() {
        let (backend, _profile) = testing::unlocked();
        let directory =
            engine::db::run_here(backend.engine(), engine::libraries::open_library_directory)
                .unwrap();
        let mut library = Preferences::default();
        library.set("devnotes.notes.arrangement", "board");
        library
            .write(&directory.join(engine::layout::PREFERENCES))
            .unwrap();
        let ctx = egui::Context::default();
        let mut state = AppState::new(compact(), true);
        state.apply_look(&ctx);
        let typed = egui::Id::unique("typed");
        ctx.data_mut(|data| data.insert_temp(typed, "psql -h prod".to_string()));
        let before = state.revision;

        state.reset_library_scope(&ctx, &backend);
        backend.settle();
        state.update(&ctx, &backend);

        assert_eq!(
            state.library_preferences.get("devnotes.notes.arrangement"),
            Some("board")
        );
        assert_eq!(state.settings.density, Density::Compact);
        assert_eq!(ctx.data(|data| data.get_temp::<String>(typed)), None);
        assert_eq!(theme::spacing(&ctx), &theme::tokens::COMPACT);
        assert!(theme::motion::reduced(&ctx));
        assert_ne!(state.revision, before);
    }

    #[test]
    fn an_action_changes_the_settings_and_writes_them_with_the_unknown_keys() {
        let (backend, profile) = testing::backend();
        let ctx = egui::Context::default();
        let mut state = AppState::new(compact(), false);

        state.apply(Action::App(AppAction::ShowArea(Area::Http)), &ctx, &backend);
        state.apply(
            Action::App(AppAction::SetAppearance(Appearance::Light)),
            &ctx,
            &backend,
        );
        backend.settle();

        let written = Preferences::read(&profile.path().join(engine::layout::PREFERENCES));
        assert_eq!(written.get("devnotes.area"), Some("http"));
        assert_eq!(written.get("devnotes.theme"), Some("light"));
        assert_eq!(written.get("devnotes.someday"), Some("kept"));
        assert_eq!(Settings::read(&written), state.settings);
    }

    #[test]
    fn a_bumped_revision_is_another_parameter() {
        let mut revision = Revision::default();
        let before = revision;

        revision.bump();

        assert_ne!(revision, before);
    }
}
