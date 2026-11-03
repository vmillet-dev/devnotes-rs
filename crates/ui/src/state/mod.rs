//! What the interface remembers between frames, apart from the drawing (plan § 3.4).

pub mod notes_query;
mod query;

pub use query::QueryCell;

use crate::action::{Action, AppAction, GlobalAction};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::gate::{Gate, GateView};
use crate::notes;
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;
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
pub struct LibraryCaches {}

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
    pub revision: Revision,
    pub notes_query: NotesQueryState,
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
            revision: Revision::default(),
            notes_query: NotesQueryState::default(),
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
        if let Some(pending) = &mut self.library_preferences_read
            && let Some(answer) = pending.poll()
        {
            self.library_preferences_read = None;
            match answer {
                Ok(preferences) => self.library_preferences = preferences,
                Err(error) => self
                    .errors
                    .report(i18n::keys::errors::LIBRARY_ACTION_FAILED, &error),
            }
        }
        if self.gate.is_open() {
            let now = self.clock.now();
            self.notes_query
                .update(ctx, backend, now, self.revision, self.gate.index);
        }
    }

    /// At the end of the frame, in the order the actions were pushed.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "the notes' actions carry what they move"
    )]
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
            Action::Gate(action) => self.gate.apply(action, &mut self.gate_view, backend),
            Action::Notes(notes::Action::Reload) => self.revision.bump(),
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
        self.caches = LibraryCaches::default();
        self.notes_query = NotesQueryState::default();
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
