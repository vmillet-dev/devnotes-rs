//! What the interface remembers between frames, apart from the drawing (plan § 3.4).

pub mod attachments;
pub mod calc_sheets;
pub mod diagrams;
pub mod drawing_exports;
pub mod drawing_pictures;
pub mod editor;
pub mod explored;
pub mod formatting;
pub mod note_revisions;
pub mod notes;
pub mod notes_query;
pub mod palette;
pub mod placeholder_fill;
pub mod presentation;
mod query;
pub mod selection;
pub mod spaces;
pub mod tags;
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
pub struct LibraryCaches {
    pub thumbnails: drawing_pictures::Thumbnails,
    /// The large pictures Notes embed.
    pub pictures: drawing_pictures::Thumbnails,
    pub diagrams: diagrams::Diagrams,
}

impl Default for LibraryCaches {
    fn default() -> Self {
        Self {
            thumbnails: drawing_pictures::Thumbnails::default(),
            pictures: drawing_pictures::Thumbnails::large(),
            diagrams: diagrams::Diagrams::default(),
        }
    }
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
    pub revisions: note_revisions::RevisionsState,
    pub selection: selection::SelectionState,
    pub undo: undo::UndoState,
    pub trash: trash::TrashState,
    pub tags: tags::TagsState,
    pub fill: placeholder_fill::FillState,
    pub palette: palette::PaletteState,
    pub palette_view: crate::palette::PaletteView,
    pub caches: LibraryCaches,
    pub status: StatusNotifier,
    pub dialog: std::sync::Arc<dyn platform::dialog::FileDialog + Send + Sync>,
    pub opener: std::sync::Arc<dyn platform::desktop::Opener + Send + Sync>,
    pub clipboard: std::sync::Arc<dyn platform::clipboard::Clipboard + Send + Sync>,
    pub attachments: attachments::AttachmentsState,
    pub exports: drawing_exports::DrawingExports,
    pub calc: calc_sheets::CalcSheets,
    pub explored: explored::ExploredState,
    pub formatting: formatting::FormattingState,
    pub presentation: presentation::PresentationState,
    /// The open library's, read with its preferences.
    pub prettier: crate::notes::prettier::PrettierSettings,
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
            revisions: note_revisions::RevisionsState::default(),
            selection: selection::SelectionState::default(),
            undo: undo::UndoState::default(),
            trash: trash::TrashState::default(),
            tags: tags::TagsState::default(),
            fill: placeholder_fill::FillState::default(),
            palette: palette::PaletteState::default(),
            palette_view: crate::palette::PaletteView::default(),
            caches: LibraryCaches::default(),
            status: StatusNotifier::default(),
            dialog: platform::dialog::system(),
            opener: platform::desktop::system(),
            clipboard: platform::clipboard::system(),
            attachments: attachments::AttachmentsState::default(),
            exports: drawing_exports::DrawingExports::default(),
            calc: calc_sheets::CalcSheets::default(),
            explored: explored::ExploredState::default(),
            formatting: formatting::FormattingState::default(),
            presentation: presentation::PresentationState::default(),
            prettier: crate::notes::prettier::PrettierSettings::default(),
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
            self.editor.open(id);
        }
        for note in answered.copied {
            self.copied(ctx, backend, note);
        }
        let time = ctx.input(|input| input.time);
        for done in answered.done {
            self.follow_in_editor(&done);
            self.undo.record(done, time);
        }
        self.undo.update(&mut self.revision, &mut self.errors);
        self.trash
            .update(ctx, backend, &mut self.revision, &mut self.errors);
        self.tags
            .update(backend, &mut self.revision, &mut self.errors);
        let filled = self.fill.update(&mut self.revision, &mut self.errors);
        for (text, said) in filled.copied {
            self.copy_out(ctx, text, said);
        }
        let pinned_first = self.notes_query.arrangement().pinned_first;
        self.palette.update(
            ctx,
            backend,
            (self.clock.now(), pinned_first, self.revision),
        );
        self.editor
            .adopt(backend, &mut self.revision, &mut self.errors);
        self.editor.update(backend, self.revision, &mut self.errors);
        self.exports.update(&mut self.status, &mut self.errors);
        if self.attachments.wants_row() {
            self.editor.write_draft(backend);
        }
        let mut reports = attachments::Reports {
            status: &mut self.status,
            errors: &mut self.errors,
            revision: &mut self.revision,
        };
        self.attachments.update(
            ctx,
            backend,
            self.editor.open_id(),
            &self.opener,
            &mut reports,
        );
        self.explored.update(backend);
        for saved in self.formatting.update() {
            self.editor
                .save_body(backend, saved.session, saved.id, saved.body);
        }
        self.presentation.update(ctx, &mut self.errors);
        for (note, sheet) in self.calc.update(&mut self.errors) {
            let text = crate::notes::calc::copy_text(&note.content, &sheet, self.language);
            self.copy_out(ctx, text, placeholder_fill::copied_note(&note));
        }
        self.follow_revisions(backend);
        for id in self.editor.take_redrawn() {
            self.caches.thumbnails.redrawn(&id);
            self.caches.pictures.redrawn(&id);
        }
        self.caches.thumbnails.update(ctx, backend);
        self.caches.pictures.update(ctx, backend);
        self.caches.diagrams.update(ctx, backend);
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

    /// The bodies kept beside the note open — none for a draft or a todo list — and a body put
    /// back, which the editor's fields start again from.
    fn follow_revisions(&mut self, backend: &Backend) {
        let kept = self
            .editor
            .opened()
            .and_then(|open| open.note().map(|note| (open.id(), note.kind)))
            .and_then(|(id, kind)| id.filter(|_| kind != engine::notes::kind::NoteKind::Checklist));
        self.revisions.follow(kept);
        if let Some(id) = self
            .revisions
            .update(backend, &mut self.revision, &mut self.errors)
        {
            self.editor.reseed_when_read(&id);
        }
    }

    /// A write made on the note open: converted, the editor's fields start again from what it now
    /// holds; deleted, the editor closes.
    fn follow_in_editor(&mut self, done: &api::undo::Reversible) {
        use api::undo::Reversible;
        let Some(open) = self.editor.open_id().map(str::to_owned) else {
            return;
        };
        match done {
            Reversible::Convert { id, .. } if *id == open => self.editor.reseed(&open),
            Reversible::Deletion { ids, .. } if ids.contains(&open) => self.editor.close(),
            _ => {}
        }
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
                self.prettier = crate::notes::prettier::PrettierSettings::read(
                    preferences.get(crate::notes::prettier::KEY),
                );
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
            // Not before a library is open: there is nothing to search.
            Action::Global(global @ (GlobalAction::Palette | GlobalAction::PaletteHere)) => {
                if self.gate.is_open() {
                    self.palette.open(backend, global == GlobalAction::Palette);
                }
            }
            Action::Palette(asked) => self.apply_palette(asked, ctx, backend),
            // The capture arrives with its own phase.
            Action::Global(GlobalAction::Capture) => {}
        }
    }

    /// The spaces and the folders: chosen, made, renamed, deleted.
    fn apply_place(&mut self, action: crate::notes::Action, backend: &Backend) {
        use crate::notes::Action as Notes;
        match action {
            Notes::ChooseSpace(id) => {
                self.spaces.choose_space(id);
                self.remember_place(backend);
            }
            Notes::ChooseFolder(id) => {
                self.spaces.choose_folder(&id);
                self.remember_place(backend);
            }
            Notes::CountSpace(id) => self.spaces.count(id),
            Notes::CreateSpace(name) => self.spaces.create_space(backend, &name),
            Notes::RenameSpace { id, name } => {
                self.spaces.rename_space(backend, id, &name);
            }
            Notes::PinSpace { id, pinned } => {
                self.spaces.pin_space(backend, id, pinned);
            }
            Notes::DeleteSpace { id, refuge } => {
                self.spaces.delete_space(backend, id, refuge);
            }
            Notes::CreateFolder {
                space_id,
                parent_id,
                name,
            } => self
                .spaces
                .create_folder(backend, space_id, parent_id, &name),
            Notes::RenameFolder { id, name } => {
                self.spaces.rename_folder(backend, id, &name);
            }
            Notes::RecolourFolder { id, colour } => {
                self.spaces.recolour_folder(backend, id, colour);
            }
            Notes::DeleteFolder(id) => self.spaces.delete_folder(backend, id),
            _ => {}
        }
    }

    /// What opens over the canvas: the undo bar, the trash, the tag manager.
    fn apply_overlay(
        &mut self,
        action: crate::notes::Action,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::notes::Action as Notes;
        match action {
            Notes::Undo => {
                self.undo.revert(backend);
            }
            Notes::DismissUndo => self.undo.dismiss(),
            Notes::DismissFolderName => self.spaces.dismiss_naming(),
            Notes::Tags(asked) => self.tags.apply(asked, backend),
            Notes::Fill(asked) => {
                if asked == crate::notes::Fill::Cancel {
                    self.palette.copied();
                }
                if let Some((text, said)) = self.fill.apply(asked, backend) {
                    self.copy_out(ctx, text, said);
                }
            }
            Notes::Trash(asked) => {
                self.trash
                    .apply(asked, backend, ctx.input(|input| input.time));
            }
            _ => {}
        }
    }

    fn apply_notes(
        &mut self,
        action: crate::notes::Action,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::notes::Action as Notes;
        match action {
            Notes::Reload => self.revision.bump(),
            Notes::SetItems { id, items } => {
                self.notes.set_items(backend, id, items);
            }
            Notes::Thumbnail(stamp) => self.caches.thumbnails.ask(backend, stamp),
            Notes::Picture(id) => {
                let stamp = drawing_pictures::Thumbnails::embedded(&id);
                self.caches.pictures.ask(backend, stamp);
            }
            Notes::Diagram(source) => self.caches.diagrams.ask(ctx, &source),
            place @ (Notes::ChooseSpace(_)
            | Notes::ChooseFolder(_)
            | Notes::CountSpace(_)
            | Notes::CreateSpace(_)
            | Notes::RenameSpace { .. }
            | Notes::PinSpace { .. }
            | Notes::DeleteSpace { .. }
            | Notes::CreateFolder { .. }
            | Notes::RenameFolder { .. }
            | Notes::RecolourFolder { .. }
            | Notes::DeleteFolder(_)) => self.apply_place(place, backend),
            Notes::Search(typed) => self.notes_query.type_search(ctx, typed),
            Notes::ClearSearch => self.notes_query.clear_search(),
            Notes::SetFilter(filter) => self.notes_query.set_filter(filter),
            Notes::Arrange(arrangement) => {
                self.notes_query.arrange(arrangement);
                arrangement.write(&mut self.library_preferences);
                drop(api::preferences::write_library(
                    backend,
                    self.library_preferences.clone(),
                ));
            }
            Notes::OpenNote(id) => self.editor.open(id),
            Notes::Editor(editing) => self.apply_editing(editing, ctx, backend),
            Notes::NewNote(kind) => self.new_note(kind),
            Notes::Pin { id, pinned } => {
                self.notes.pin(backend, vec![id], pinned);
            }
            Notes::Prioritise { id, priority } => {
                self.notes.prioritise(backend, vec![id], priority);
            }
            Notes::File { id, folder } => {
                self.notes.file(backend, vec![id], folder);
            }
            Notes::MoveTo { id, space } => {
                self.notes.move_to(backend, vec![id], space);
            }
            Notes::Delete(id) => self.notes.delete(backend, vec![id]),
            Notes::Check(id) => self.selection.toggle(&id),
            Notes::CheckRange { id, anchor } => {
                let order = self.on_screen();
                let order: Vec<&str> = order.iter().map(String::as_str).collect();
                self.selection.check_range(&order, anchor.as_deref(), &id);
            }
            Notes::ClearSelection => {
                self.selection.clear();
            }
            Notes::Batch(batch) => self.batch(batch, backend),
            Notes::Facet(facet) => self.notes_query.press(facet),
            overlay @ (Notes::Undo
            | Notes::DismissUndo
            | Notes::Tags(_)
            | Notes::Fill(_)
            | Notes::DismissFolderName
            | Notes::Trash(_)) => {
                self.apply_overlay(overlay, ctx, backend);
            }
            Notes::Duplicate { id, title } => {
                self.notes.duplicate(backend, id, title);
            }
            Notes::Copy(id) => self.notes.copy(backend, id),
            Notes::Canvas(key) => self.canvas_key(key, ctx, backend),
            Notes::FocusSearch => {
                self.notes_view.focus_search = true;
                // Taken by the next frame, which nothing else may ask for.
                ctx.request_repaint();
            }
        }
    }

    /// What the editor asks: a draft changes where it is, a note written goes through the same
    /// writes as its card, the undo offered.
    fn apply_editing(
        &mut self,
        editing: crate::notes::Editing,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::notes::Editing;
        let id = self.editor.open_id().map(str::to_owned);
        match editing {
            Editing::Save { session, edit } => self.save_formatted(backend, session, edit),
            Editing::Format { session, request } => self.formatting.ask(backend, session, request),
            Editing::Prettier(settings) => self.set_prettier(settings, backend),
            Editing::Export(export) => self.exports.ask(backend, &self.dialog, export),
            Editing::Fields(asked) => self.apply_fields(asked, ctx, backend),
            Editing::Attachment(asked) => self.ask_attachment(asked, backend),
            Editing::Explore { session, query } => self.explored.want(session, query),
            Editing::OpenLink(url) => self.open_link(&url),
            Editing::Evaluate { session, text } => {
                let today = self.clock.now().with_timezone(&chrono::Local).date_naive();
                self.calc.ask(backend, session, text, today);
            }
            Editing::Pictures {
                session,
                content,
                palette,
                missing,
            } => self
                .editor
                .picture(backend, session, (content, palette), missing),
            Editing::Pin(pinned) => {
                if !self
                    .editor
                    .change_draft(backend, |note| note.pinned = pinned)
                    && let Some(id) = id
                {
                    self.notes.pin(backend, vec![id], pinned);
                }
            }
            Editing::Prioritise(priority) => {
                if !self
                    .editor
                    .change_draft(backend, |note| note.priority = priority)
                    && let Some(id) = id
                {
                    self.notes.prioritise(backend, vec![id], priority);
                }
            }
            Editing::MoveTo(space) => {
                let into = space.clone();
                let moved = self.editor.change_draft(backend, |note| {
                    note.space_id = into;
                    note.folder_id = None;
                });
                if !moved && let Some(id) = id {
                    self.notes.move_to(backend, vec![id], space);
                }
            }
            Editing::File(folder) => {
                let into = folder.clone();
                if !self
                    .editor
                    .change_draft(backend, |note| note.folder_id = into)
                    && let Some(id) = id
                {
                    self.notes.file(backend, vec![id], folder);
                }
            }
            Editing::Convert {
                session,
                into,
                edit,
            } => {
                self.editor.save(backend, session, edit);
                match id {
                    Some(id) => self.notes.convert(backend, id, into),
                    None => self.editor.convert_draft(into),
                }
            }
            Editing::Delete => match id {
                Some(id) => self.notes.delete(backend, vec![id]),
                None => self.editor.close(),
            },
            Editing::Copy => {
                if let Some(id) = id {
                    self.notes.copy(backend, id);
                }
            }
            Editing::Close => self.editor.close(),
            Editing::Present { title, markdown } => {
                self.presentation.ask(backend, title, markdown);
            }
            Editing::StopPresenting => self.presentation.close(ctx),
            Editing::Fullscreen(on) => {
                self.settings.editor_fullscreen = on;
                self.save_settings(backend);
            }
            Editing::Revisions(asked) => self.apply_revisions(asked, backend),
            Editing::FoldHeader(folded) => {
                self.settings.editor_header_folded = folded;
                self.save_settings(backend);
            }
        }
        ctx.request_repaint();
    }

    fn ask_attachment(&mut self, asked: attachments::Asked, backend: &Backend) {
        let moment = self.clock.now().format("capture-%Y-%m-%d-%H-%M-%S");
        self.attachments
            .ask(asked, backend, &self.dialog, moment.to_string());
    }

    fn apply_fields(
        &mut self,
        asked: crate::notes::Fields,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::notes::Fields;
        match asked {
            Fields::Keep(values) => {
                if let Some(id) = self.editor.open_id() {
                    self.fill.keep(backend, id.to_owned(), values);
                }
            }
            Fields::Preview {
                session,
                content,
                values,
            } => self.fill.ask_preview(backend, session, content, values),
            Fields::Copy { content, values } => self.fill.copy(backend, content, values),
            Fields::CopyRaw(text) => {
                let note = self.editor.opened().and_then(|open| open.note());
                if let Some(said) = note.map(placeholder_fill::copied_note) {
                    self.copy_out(ctx, text, said);
                }
            }
            Fields::Fold(open) => {
                self.settings.editor_fields_open = open;
                self.save_settings(backend);
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
        if self.notes_query.has_user_filters() {
            self.notes_query.clear_filters();
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
    /// A note with `{{fields}}` asks for them first.
    fn copied(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        note: engine::notes::model::DisplayNote,
    ) {
        if note.kind == engine::notes::kind::NoteKind::Drawing {
            self.status
                .report(i18n::Message::new(i18n::keys::notes::DRAWING_NOT_COPIED));
            return;
        }
        if !note.placeholders.is_empty() {
            self.fill.open(note);
            return;
        }
        if note.kind == engine::notes::kind::NoteKind::Calc {
            let today = self.clock.now().with_timezone(&chrono::Local).date_naive();
            self.calc.copy(backend, note, today);
            return;
        }
        let text = note
            .copy_text
            .clone()
            .unwrap_or_else(|| note.content.clone());
        self.copy_out(ctx, text, placeholder_fill::copied_note(&note));
    }

    /// Onto the clipboard, and said; a copy asked from the palette closes it.
    fn copy_out(&mut self, ctx: &egui::Context, text: String, said: i18n::Message) {
        ctx.copy_text(text);
        if self.palette.copied() {
            self.palette.close();
        }
        self.status.report(said);
    }

    fn apply_palette(
        &mut self,
        asked: crate::palette::Asked,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::palette::Asked;
        use crate::palette::options::Chosen;
        match asked {
            Asked::Type(text) => self.palette.type_text(text, ctx.input(|input| input.time)),
            Asked::Chip(chip) => self.palette.add_chip(chip),
            Asked::Unchip(at) => self.palette.remove_chip(at),
            Asked::Close => self.palette.close(),
            Asked::Copy(id) => {
                self.palette.copying();
                self.notes.copy(backend, id);
            }
            Asked::Choose(chosen) => {
                self.palette.close();
                let area = match chosen {
                    Chosen::Command(crate::palette::commands::Command::Area(area)) => area,
                    _ => Area::Notes,
                };
                self.apply(Action::App(AppAction::ShowArea(area)), ctx, backend);
                self.run_chosen(chosen, backend);
            }
        }
    }

    fn run_chosen(&mut self, chosen: crate::palette::options::Chosen, backend: &Backend) {
        use crate::palette::commands::Command;
        use crate::palette::options::Chosen;
        match chosen {
            Chosen::Note(id) => self.editor.open(id),
            Chosen::Space(id) => {
                self.spaces.choose_space(Some(id));
                self.remember_place(backend);
            }
            Chosen::Command(Command::Create(kind)) => self.new_note(Some(kind)),
            Chosen::Command(Command::Folder) => self.spaces.ask_name(),
            Chosen::Command(Command::Area(_)) => {}
            Chosen::Create(title) => {
                if let Some(place) = self.new_place() {
                    let kind = engine::notes::kind::NoteKind::Snippet;
                    self.editor
                        .draft_titled(backend, (kind, place.0, place.1), title);
                }
            }
        }
    }

    /// A modal over the canvas takes its keys: the canvas's stand aside.
    pub fn modal_open(&self) -> bool {
        self.editor.is_open()
            || self.trash.is_open()
            || self.tags.is_open()
            || self.fill.is_open()
            || self.palette.is_open()
            || self.spaces.naming()
    }

    /// In the space shown, or the first one for « all spaces », and the folder shown.
    fn new_note(&mut self, kind: Option<engine::notes::kind::NoteKind>) {
        let Some((space, folder)) = self.new_place() else {
            return;
        };
        let kind = kind.unwrap_or(engine::notes::kind::NoteKind::Snippet);
        self.editor.draft(kind, space, folder);
    }

    /// Where a new note goes: the space and folder shown, or the first space for « all »; said
    /// when there is none.
    fn new_place(&mut self) -> Option<(String, Option<String>)> {
        let space = self
            .spaces
            .active_space()
            .map(str::to_owned)
            .or_else(|| self.spaces.spaces().first().map(|space| space.id.clone()));
        let Some(space) = space else {
            self.errors
                .notify(i18n::Message::new(i18n::keys::errors::SPACE_REQUIRED));
            return None;
        };
        Some((space, self.spaces.active_folder().map(str::to_owned)))
    }

    /// The settings into the application's file, through the db queue, the other keys kept.
    fn apply_revisions(&mut self, asked: crate::notes::Revisions, backend: &Backend) {
        use crate::notes::Revisions;
        match asked {
            Revisions::Toggle => self.revisions.toggle(),
            Revisions::Look(id) => self.revisions.look(backend, &id),
            Revisions::Back => self.revisions.close_preview(),
            Revisions::Restore => self.revisions.restore(backend),
        }
    }

    /// The desktop's own browser: a URL goes the way a file does.
    fn open_link(&self, url: &str) {
        drop(self.opener.open(std::path::Path::new(url)));
    }

    fn set_prettier(
        &mut self,
        settings: crate::notes::prettier::PrettierSettings,
        backend: &Backend,
    ) {
        self.prettier = settings;
        self.library_preferences
            .set(crate::notes::prettier::KEY, &settings.written());
        drop(api::preferences::write_library(
            backend,
            self.library_preferences.clone(),
        ));
    }

    /// Formatted first when the library asks for it: the body is held back and written once,
    /// formatted or, refused, as it was typed. A draft never written is saved as it is.
    fn save_formatted(&mut self, backend: &Backend, session: u64, mut edit: editor::Edit) {
        let note = self.editor.opened().and_then(editor::Opened::note);
        let asked = note
            .filter(|_| self.prettier.format_on_save)
            .and_then(|note| {
                let parser = crate::notes::language::formatter(note.language)?;
                let Some(editor::Body::Content(text)) = &edit.body else {
                    return None;
                };
                let changed = *text != note.content && !note.id.is_empty();
                changed.then(|| {
                    let indent = crate::notes::language::indent_unit(note.language);
                    let request = text::format::Request {
                        text: text.clone(),
                        cursor: 0,
                        parser,
                        options: self.prettier.options(indent),
                    };
                    (note.id.clone(), request)
                })
            });
        if let Some((id, request)) = asked {
            edit.body = None;
            self.formatting.save(backend, session, id, request);
        }
        self.editor.save(backend, session, edit);
    }

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
        self.prettier = crate::notes::prettier::PrettierSettings::default();
        self.library_preferences_read = Some(api::preferences::library(backend));
        self.seeding = None;
        self.presentation.close(ctx);
        self.caches = LibraryCaches::default();
        self.notes_query = NotesQueryState::default();
        self.spaces = spaces::SpacesState::default();
        self.editor = editor::EditorState::default();
        self.revisions = note_revisions::RevisionsState::default();
        self.selection = selection::SelectionState::default();
        self.undo = undo::UndoState::default();
        self.trash = trash::TrashState::default();
        self.tags = tags::TagsState::default();
        self.fill = placeholder_fill::FillState::default();
        self.palette = palette::PaletteState::default();
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
