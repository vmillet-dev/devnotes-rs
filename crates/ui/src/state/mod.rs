//! What the interface remembers between frames, apart from the drawing (plan § 3.4).

pub mod drawing_pictures;
pub mod editor;
pub mod notes;
pub mod notes_query;
mod query;
pub mod selection;
pub mod spaces;
pub mod trash;
pub mod undo;

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
    pub selection: selection::SelectionState,
    pub undo: undo::UndoState,
    pub trash: trash::TrashState,
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
            selection: selection::SelectionState::default(),
            undo: undo::UndoState::default(),
            trash: trash::TrashState::default(),
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
        let answered = self.notes.update(&mut self.revision, &mut self.errors);
        if let Some(id) = answered.open {
            self.editor.open(backend, id);
        }
        for note in answered.copied {
            self.copied(ctx, &note);
        }
        let time = ctx.input(|input| input.time);
        for done in answered.done {
            self.undo.record(done, time);
        }
        self.undo.update(&mut self.revision, &mut self.errors);
        self.trash
            .update(ctx, backend, &mut self.revision, &mut self.errors);
        self.editor
            .update(backend, &mut self.revision, &mut self.errors);
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
            Action::App(AppAction::ToggleRail) => {
                self.settings.rail_shown = !self.settings.rail_shown;
                self.save_settings(backend);
            }
            Action::App(AppAction::SetRailWidth(width)) => {
                self.settings.rail_width = width;
                self.save_settings(backend);
            }
            Action::Gate(action) => self.gate.apply(action, &mut self.gate_view, backend),
            Action::Notes(action) => self.apply_notes(action, ctx, backend),
            Action::Global(GlobalAction::Quit) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::Global(GlobalAction::NewNote) => self.new_note(None),
            // The capture and the palette arrive with the notes area.
            Action::Global(
                GlobalAction::Capture | GlobalAction::Palette | GlobalAction::PaletteHere,
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
            crate::notes::Action::SaveNote {
                session,
                title,
                body,
            } => self.editor.save(backend, session, title, body),
            crate::notes::Action::NewNote(kind) => self.new_note(kind),
            crate::notes::Action::Pin { id, pinned } => {
                self.notes.pin(backend, vec![id], pinned);
            }
            crate::notes::Action::Prioritise { id, priority } => {
                self.notes.prioritise(backend, vec![id], priority);
            }
            crate::notes::Action::File { id, folder } => {
                self.notes.file(backend, vec![id], folder);
            }
            crate::notes::Action::MoveTo { id, space } => {
                self.notes.move_to(backend, vec![id], space);
            }
            crate::notes::Action::Delete(id) => self.notes.delete(backend, vec![id]),
            crate::notes::Action::Check(id) => self.selection.toggle(&id),
            crate::notes::Action::CheckRange { id, anchor } => {
                let order = self.on_screen();
                let order: Vec<&str> = order.iter().map(String::as_str).collect();
                self.selection.check_range(&order, anchor.as_deref(), &id);
            }
            crate::notes::Action::ClearSelection => {
                self.selection.clear();
            }
            crate::notes::Action::Batch(batch) => self.batch(batch, backend),
            crate::notes::Action::Undo => {
                self.undo.revert(backend);
            }
            crate::notes::Action::DismissUndo => self.undo.dismiss(),
            crate::notes::Action::Trash(asked) => {
                self.trash
                    .apply(asked, backend, ctx.input(|input| input.time));
            }
            crate::notes::Action::Duplicate { id, title } => {
                self.notes.duplicate(backend, id, title);
            }
            crate::notes::Action::Copy(id) => self.notes.copy(backend, id),
            crate::notes::Action::Canvas(key) => self.canvas_key(key, ctx, backend),
            crate::notes::Action::CloseNote => self.editor.close(),
            crate::notes::Action::FocusSearch => {
                self.notes_view.focus_search = true;
                // Taken by the next frame, which nothing else may ask for.
                ctx.request_repaint();
            }
        }
    }

    /// The notes on screen, in the order they are drawn.
    pub fn on_screen(&self) -> Vec<String> {
        self.notes_query.view().map_or_else(Vec::new, |view| {
            view.cards().map(|card| card.id.clone()).collect()
        })
    }

    /// The ticked notes still on screen; nothing is asked of the core for none.
    fn batch(&mut self, batch: crate::notes::Batch, backend: &Backend) {
        use crate::notes::Batch;
        let order = self.on_screen();
        let order: Vec<&str> = order.iter().map(String::as_str).collect();
        let ids = self.selection.checked_in(&order);
        if ids.is_empty() {
            return;
        }
        match batch {
            Batch::MoveTo(space) => self.notes.move_to(backend, ids, space),
            Batch::File(folder) => self.notes.file(backend, ids, folder),
            Batch::Prioritise(priority) => self.notes.prioritise(backend, ids, priority),
            Batch::Tag(tag) => self.notes.tag(backend, ids, tag),
            Batch::Untag(tag) => self.notes.untag(backend, ids, tag),
            Batch::Pin(pinned) => self.notes.pin(backend, ids, pinned),
            Batch::Delete => {
                self.notes.delete(backend, ids);
                self.selection.clear();
            }
        }
    }

    /// A key of the canvas: the card it acts on is the one with the focus, as last drawn.
    fn canvas_key(
        &mut self,
        key: crate::notes::canvas::shortcuts::CanvasKey,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::notes::canvas::shortcuts::CanvasKey;
        let focused = self.notes_view.canvas.focused.clone();
        let note = focused.as_deref().and_then(|id| {
            self.notes_query
                .view()
                .and_then(|view| view.cards().find(|card| card.id == id))
        });
        let pinned = note.map(|card| card.pinned);
        match key {
            CanvasKey::New(kind) => self.new_note(Some(kind)),
            CanvasKey::Move(direction) => {
                let Some(view) = self.notes_query.view() else {
                    return;
                };
                let sections: Vec<Vec<&str>> = view
                    .sections
                    .iter()
                    .map(|section| section.cards.iter().map(|card| card.id.as_str()).collect())
                    .collect();
                let columns = self.notes_view.canvas.columns;
                let to = crate::notes::canvas::grid::neighbour(
                    &sections,
                    columns,
                    focused.as_deref(),
                    direction,
                );
                if let Some(to) = to {
                    self.notes_view.canvas.request = Some(to.to_owned());
                    ctx.request_repaint();
                }
            }
            CanvasKey::Copy => {
                if let Some(id) = focused {
                    self.notes.copy(backend, id);
                }
            }
            CanvasKey::Pin => {
                if let (Some(id), Some(pinned)) = (focused, pinned) {
                    self.notes.pin(backend, vec![id], !pinned);
                }
            }
            CanvasKey::Priority(priority) => {
                if let Some(id) = focused {
                    self.notes.prioritise(backend, vec![id], priority);
                }
            }
            CanvasKey::Check => {
                if let Some(id) = focused {
                    self.selection.toggle(&id);
                }
            }
            CanvasKey::Trash => {
                if let Some(id) = focused
                    && self.selection.arm(&id)
                {
                    self.notes.delete(backend, vec![id]);
                }
            }
            CanvasKey::Undo => {
                self.undo.revert(backend);
            }
            CanvasKey::StepBack => self.step_back(ctx, backend),
        }
    }

    /// Escape, down the ladder and one rung at a time: what is armed, what the undo bar offers
    /// (only while it is on screen: Escape must not quietly rewrite the corpus), the selection,
    /// the search, up
    /// a folder — leaving is the biggest step, so it comes last.
    fn step_back(&mut self, ctx: &egui::Context, backend: &Backend) {
        if self.selection.disarm() {
            return;
        }
        if self.undo.banner(ctx.input(|input| input.time)).is_some() {
            self.undo.revert(backend);
            return;
        }
        if self.selection.clear() {
            return;
        }
        if !self.notes_query.typed().is_empty() {
            self.notes_query.clear_search();
            return;
        }
        let Some(open) = self.spaces.active_folder() else {
            return;
        };
        let parent = self
            .spaces
            .folders()
            .iter()
            .find(|folder| folder.id == open)
            .map(|folder| (folder.space_id.clone(), folder.parent_id.clone()));
        match parent {
            Some((_, Some(parent))) => self.spaces.choose_folder(&parent),
            Some((space, None)) => self.spaces.choose_space(Some(space)),
            None => {}
        }
    }

    /// What copying yields, onto the clipboard, and said: what was copied, by its title.
    fn copied(&mut self, ctx: &egui::Context, note: &engine::notes::model::DisplayNote) {
        let text = note
            .copy_text
            .clone()
            .unwrap_or_else(|| note.content.clone());
        ctx.copy_text(text);
        let message = if note.title.is_empty() {
            i18n::Message::new(i18n::keys::notes::COPIED_UNTITLED)
        } else {
            i18n::Message::new(i18n::keys::notes::COPIED_NOTE).with("title", note.title.as_str())
        };
        self.status.report(message);
    }

    /// In the space shown, or the first one for « all spaces », and the folder shown.
    fn new_note(&mut self, kind: Option<engine::notes::kind::NoteKind>) {
        let space = self
            .spaces
            .active_space()
            .map(str::to_owned)
            .or_else(|| self.spaces.spaces().first().map(|space| space.id.clone()));
        let Some(space) = space else {
            self.errors
                .notify(i18n::Message::new(i18n::keys::errors::SPACE_REQUIRED));
            return;
        };
        let folder = self.spaces.active_folder().map(str::to_owned);
        let kind = kind.unwrap_or(engine::notes::kind::NoteKind::Snippet);
        self.editor.draft(kind, space, folder);
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
        self.selection = selection::SelectionState::default();
        self.undo = undo::UndoState::default();
        self.trash = trash::TrashState::default();
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
