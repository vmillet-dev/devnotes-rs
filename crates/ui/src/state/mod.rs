//! What the interface remembers between frames, apart from the drawing (plan § 3.4).

pub mod attachments;
pub mod backups;
pub mod board;
pub mod calc_sheets;
pub mod diagrams;
pub mod drawing_exports;
pub mod drawing_pictures;
pub mod editor;
pub mod explored;
pub mod folders;
pub mod formatting;
pub mod help;
pub mod http;
pub mod libraries;
pub mod note_revisions;
pub mod notes;
pub mod notes_query;
pub mod palette;
pub mod placeholder_fill;
pub mod presentation;
pub(crate) mod query;
pub mod security;
pub mod selection;
pub mod spaces;
pub mod tags;
pub mod tools;
pub mod transfer;
pub mod trash;
pub mod undo;

pub use query::QueryCell;

use engine::folders::board::PreviewLines;

use crate::action::{Action, AppAction, GlobalAction};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::gate::{Gate, GateView};
use crate::services::errors::ErrorNotifier;
use crate::services::preferences::Preferences;
use crate::services::samples;
use crate::services::settings::Settings;
use crate::services::status::StatusNotifier;
use crate::services::time::{Clock, SystemClock};
use crate::theme;
use crate::widgets::Layer;
use notes_query::NotesQueryState;

/// What the editor opens once the note open has closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorNext {
    Note(String),
    /// Reached by J or K.
    Stepped(String),
    /// A new note of that kind, or of the preferred one.
    Draft(Option<engine::notes::kind::NoteKind>),
}

/// The library's: its board's loose cards are stored in pixels, at that card's height.
pub const PREVIEW_LINES_KEY: &str = "devnotes.notes.previewLines";

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
    pub board: board::BoardState,
    pub spaces: spaces::SpacesState,
    pub folders: folders::FoldersState,
    /// The area's view: only its drawing reads it (D13).
    pub notes_view: crate::notes::View,
    pub notes: notes::NotesState,
    pub editor: editor::EditorState,
    /// The window waits for the note open to close.
    pub closing_editor: bool,
    /// What was asked for while a note is open beside the cards: opened once that one has
    /// written its fields and closed, by its own closing path.
    pub editor_next: Option<EditorNext>,
    /// « Quit », and not the close button: the window goes whatever the tray preference says.
    pub quitting: bool,
    pub revisions: note_revisions::RevisionsState,
    /// What a Note open can embed, read again at each revision: a drawing may be new.
    pub drawing_choices: QueryCell<Revision, Vec<engine::notes::drawing::DrawingRef>>,
    pub selection: selection::SelectionState,
    pub undo: undo::UndoState,
    pub trash: trash::TrashState,
    pub tags: tags::TagsState,
    pub fill: placeholder_fill::FillState,
    pub palette: palette::PaletteState,
    pub palette_view: crate::palette::PaletteView,
    pub caches: LibraryCaches,
    pub http: http::HttpState,
    /// The HTTP area's view: only its drawing reads it (D13).
    pub http_view: crate::http::View,
    pub tools: tools::ToolsState,
    /// The Tools area's view, each tool's session among it: only its drawing reads it (D13).
    pub tools_view: crate::tools::View,
    pub transfer: transfer::TransferState,
    pub libraries: libraries::LibrariesState,
    pub security: security::SecurityState,
    pub backups: backups::BackupsState,
    /// Not the library's: it outlives a switch, and is read at the gate too.
    pub help: help::HelpState,
    pub updates: crate::services::updates::UpdateState,
    pub global_keys: crate::services::global_keys::GlobalKeys,
    pub autostart: crate::services::autostart::AutostartState,
    pub notifier: std::sync::Arc<dyn platform::notify::Notifier + Send + Sync>,
    /// The bar's menus and what they open: only their drawing reads it (D13).
    pub titlebar_view: crate::titlebar::View,
    /// The modals the bar's menus open, in the order they are drawn.
    pub dialogs: crate::widgets::DialogStack,
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
    /// How many lines a card shows, the library's (`devnotes.notes.previewLines`).
    pub preview_lines: PreviewLines,
    pub errors: ErrorNotifier,
    pub clock: Box<dyn Clock>,
    /// The system's wish; the look adds nothing to it.
    pub reduced_motion: bool,
}

/// Out of sight, for the tray or a key to call back. Nothing hides a window under native
/// Wayland: it minimises there (E12).
pub(crate) fn put_away(ctx: &egui::Context) {
    ctx.send_viewport_cmd(if platform::window::is_native_wayland() {
        egui::ViewportCommand::Minimized(true)
    } else {
        egui::ViewportCommand::Visible(false)
    });
}

impl AppState {
    pub fn new(preferences: Preferences, reduced_motion: bool) -> Self {
        let settings = Settings::read(&preferences);
        let tools = tools::ToolsState::read(&preferences);
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
            board: board::BoardState::default(),
            spaces: spaces::SpacesState::default(),
            folders: folders::FoldersState::default(),
            notes_view: crate::notes::View::default(),
            notes: notes::NotesState::default(),
            editor: editor::EditorState::default(),
            closing_editor: false,
            editor_next: None,
            quitting: false,
            revisions: note_revisions::RevisionsState::default(),
            drawing_choices: QueryCell::default(),
            selection: selection::SelectionState::default(),
            undo: undo::UndoState::default(),
            trash: trash::TrashState::default(),
            tags: tags::TagsState::default(),
            fill: placeholder_fill::FillState::default(),
            palette: palette::PaletteState::default(),
            palette_view: crate::palette::PaletteView::default(),
            caches: LibraryCaches::default(),
            http: http::HttpState::default(),
            http_view: crate::http::View::default(),
            tools,
            tools_view: crate::tools::View::default(),
            transfer: transfer::TransferState::default(),
            libraries: libraries::LibrariesState::default(),
            security: security::SecurityState::default(),
            backups: backups::BackupsState::default(),
            help: help::HelpState::default(),
            updates: crate::services::updates::UpdateState::default(),
            global_keys: crate::services::global_keys::GlobalKeys::default(),
            autostart: crate::services::autostart::AutostartState::default(),
            notifier: std::sync::Arc::new(platform::notify::NoNotifier),
            titlebar_view: crate::titlebar::View::default(),
            dialogs: crate::widgets::DialogStack::default(),
            status: StatusNotifier::default(),
            dialog: std::sync::Arc::new(platform::dialog::NoDialog),
            opener: platform::desktop::system(),
            clipboard: platform::clipboard::system(),
            attachments: attachments::AttachmentsState::default(),
            exports: drawing_exports::DrawingExports::default(),
            calc: calc_sheets::CalcSheets::default(),
            explored: explored::ExploredState::default(),
            formatting: formatting::FormattingState::default(),
            presentation: presentation::PresentationState::default(),
            prettier: crate::notes::prettier::PrettierSettings::default(),
            preview_lines: PreviewLines::default(),
            errors: ErrorNotifier::default(),
            clock: Box::new(SystemClock),
            reduced_motion,
        }
    }

    /// Once a frame, before drawing: the answers that arrived are adopted.
    pub fn update(&mut self, ctx: &egui::Context, backend: &Backend) {
        self.dialogs.begin_frame(ctx);
        self.help.update();
        self.updates.update(&self.settings, &mut self.errors);
        let offered = self.updates.offered().is_some();
        let prompt = crate::banners::update_prompt::id();
        self.dialogs.follow(ctx, Layer::Update, prompt, offered);
        if self
            .gate
            .update(backend, &mut self.status, &mut self.errors)
        {
            self.reset_library_scope(ctx, backend);
        }
        if self.autostart.update(backend, &mut self.settings) {
            self.save_settings(backend);
        }
        if self.updates.take_leaving() {
            self.apply(Action::Global(GlobalAction::Quit), ctx, backend);
        }
        self.adopt_library_preferences(backend);
        self.adopt_seeding(backend);
        if self.gate.is_open() && self.library_preferences_read.is_none() && self.seeding.is_none()
        {
            self.update_library(ctx, backend);
        }
    }

    /// What reads the open library, once its preferences are in and its samples seeded.
    pub fn update_library(&mut self, ctx: &egui::Context, backend: &Backend) {
        let answered = self.notes.update(&mut self.revision, &mut self.errors);
        if let Some(id) = answered.open {
            self.open_in_editor(EditorNext::Note(id));
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
        if let Some(arranged) = self
            .board
            .write(ctx, backend, &mut self.revision, &mut self.errors)
        {
            self.undo.record(arranged, time);
        }
        self.trash
            .update(ctx, backend, &mut self.revision, &mut self.errors);
        self.tags
            .update(backend, &mut self.revision, &mut self.errors);
        let filled = self.fill.update(&mut self.revision, &mut self.errors);
        for (text, said) in filled.copied {
            self.copy_out(ctx, backend, text, said);
        }
        let pinned_first = self.settings.show_pinned_first;
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
            let text = crate::notes::calc::copy_text(&note.content, &sheet, self.calc_format());
            self.copy_out(ctx, backend, text, placeholder_fill::copied_note(&note));
        }
        self.follow_revisions(backend);
        self.follow_drawings(backend);
        self.update_area_or_exit(ctx, backend, time);
        self.update_menus(ctx, backend);
        for id in self.editor.take_redrawn() {
            self.caches.thumbnails.redrawn(&id);
            self.caches.pictures.redrawn(&id);
        }
        self.caches.thumbnails.update(ctx, backend);
        self.caches.pictures.update(ctx, backend);
        self.caches.diagrams.update(ctx, backend);
        let mut moved = self
            .spaces
            .update(backend, &mut self.revision, &mut self.errors);
        if moved {
            self.folders.leave();
        }
        let answered = self
            .folders
            .update(backend, &mut self.revision, &mut self.errors);
        for done in answered.done {
            self.undo.record(done, time);
        }
        moved |= answered.moved;
        if moved {
            self.remember_place(backend);
        }
        let now = self.clock.now();
        let folder = self.active_folder().map(str::to_owned);
        let place = (self.spaces.active_space(), folder.as_deref());
        self.notes_query
            .update(ctx, backend, now, place, self.revision);
        if let Some(place) = self.board_place() {
            let asked = (place, self.notes_query.criteria(), self.preview_lines);
            self.board.update(backend, now, asked, self.revision);
        }
    }

    /// What the bar's menus set going.
    fn update_menus(&mut self, ctx: &egui::Context, backend: &Backend) {
        self.libraries
            .update(backend, &mut self.status, &mut self.errors);
        self.security.update(&mut self.status, &mut self.errors);
        self.backups.update(&mut self.errors);
        if self.security.take_changed() {
            let change = crate::titlebar::preferences::change_id();
            self.dialogs.close(ctx, change);
        }
        if self.http.take_switch()
            && let Some(goal) = self.libraries.take_goal()
        {
            self.switch_library(goal, backend);
        }
        let mut reports = attachments::Reports {
            status: &mut self.status,
            errors: &mut self.errors,
            revision: &mut self.revision,
        };
        let now = self.clock.now();
        self.transfer
            .update(ctx, backend, &self.dialog, now, &mut reports);
        let asking = self.transfer.prompt().is_some();
        let prompt = crate::gate::prompt::id();
        self.dialogs.follow(ctx, Layer::Passphrase, prompt, asking);
    }

    /// The board, when it is what the canvas shows: the space's, or the open folder's when it
    /// holds folders. One that holds none is a flat grid whatever the switch says.
    pub fn board_place(&self) -> Option<board::Place> {
        let space_id = self.spaces.active_space()?;
        if board::Mode::read(&self.library_preferences, space_id) != board::Mode::Board {
            return None;
        }
        let root_id = self.active_folder();
        if root_id.is_some_and(|root| !self.folders.has_children(root)) {
            return None;
        }
        Some(board::Place {
            space_id: space_id.to_owned(),
            root_id: root_id.map(str::to_owned),
        })
    }

    /// On the board on screen: a gesture's result is staged and written behind the debounce.
    fn board_gesture(
        &mut self,
        gesture: crate::notes::BoardGesture,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::notes::BoardGesture;
        let Some(place) = self.board_place() else {
            return;
        };
        let now = ctx.input(|input| input.time);
        match gesture {
            BoardGesture::DropCard {
                note_id,
                folder,
                at,
            } => self.board.drop_card(
                backend,
                &place,
                (note_id, folder),
                (at, now),
                self.preview_lines,
            ),
            BoardGesture::MoveZone { folder_id, frame } => {
                self.board.move_zone(&place, folder_id, frame, now);
            }
            BoardGesture::DrawZone(frame) => self.board.propose_zone(frame),
            BoardGesture::Band(ids) => self.selection.check_all(ids),
            BoardGesture::FileOnRail { note_id, folder } => {
                self.notes
                    .file(backend, (vec![note_id], folder), self.preview_lines);
            }
            BoardGesture::SelectFolder(id) => {
                let held: Vec<String> = self
                    .board
                    .view()
                    .into_iter()
                    .flat_map(|view| &view.zones)
                    .filter(|zone| zone.folder.id == id)
                    .flat_map(|zone| zone.notes.iter().map(|entry| entry.note.id.clone()))
                    .collect();
                self.selection.check_all(held);
            }
        }
    }

    /// The board on screen put back in order; nothing while the list shows.
    fn tidy(&mut self, backend: &Backend, scope: engine::folders::board::BoardScope) {
        if let Some(place) = self.board_place() {
            self.board
                .arrange(backend, &place, scope, self.preview_lines);
        }
    }

    /// The last write put back; an arrangement taken back pans the ground back to where it was.
    fn revert_last(&mut self, backend: &Backend) {
        if matches!(
            self.undo.last(),
            Some(api::undo::Reversible::Arrangement { .. })
        ) {
            self.board.restored();
        }
        self.undo.revert(backend, self.preview_lines);
    }

    /// The folder open, `None` for every one: a folder of another space than the one shown is
    /// « every folder ».
    pub fn active_folder(&self) -> Option<&str> {
        self.folders.active(self.spaces.active_space())
    }

    /// Under `parent` or at the top of `space`. Refused and said when it cannot go there — into
    /// itself or what it holds, out of its space — by the rules of `Lineage`, the core's.
    fn move_folder(
        &mut self,
        backend: &Backend,
        id: String,
        (parent, space): (Option<String>, String),
    ) {
        use engine::folders::model::Lineage;
        let Some(folder) = self.folders.find(&id) else {
            return;
        };
        if folder.parent_id == parent {
            return;
        }
        let name = folder.name.clone();
        let lineage = Lineage::of(self.folders.list());
        let allowed =
            folder.space_id == space && lineage.check_move(&id, parent.as_deref()).is_ok();
        if !allowed {
            let refused = i18n::Message::new(i18n::keys::sidebar::moved::refused::THERE);
            self.status.report(refused.with("name", name.as_str()));
            return;
        }
        self.folders.move_folder(backend, id, parent);
    }

    /// The rail's keyboard twin of a drag: up a level, or into the sibling above it.
    fn nudge_folder(&mut self, backend: &Backend, id: &str, outwards: bool) {
        use i18n::keys::sidebar::moved::refused;
        let Some(folder) = self.folders.find(id).cloned() else {
            return;
        };
        let refused = |key| i18n::Message::new(key).with("name", folder.name.as_str());
        let target = if outwards {
            let Some(parent) = &folder.parent_id else {
                self.status.report(refused(refused::AT_TOP));
                return;
            };
            self.folders
                .find(parent)
                .and_then(|parent| parent.parent_id.clone())
        } else {
            let mut siblings: Vec<_> = self
                .folders
                .list()
                .iter()
                .filter(|other| {
                    other.space_id == folder.space_id && other.parent_id == folder.parent_id
                })
                .collect();
            siblings.sort_by_key(|other| api::folders::sort_key(&other.name));
            let at = siblings.iter().position(|other| other.id == folder.id);
            let Some(above) = at
                .and_then(|at| at.checked_sub(1))
                .map(|above| siblings[above])
            else {
                self.status.report(refused(refused::NO_FOLDER_ABOVE));
                return;
            };
            Some(above.id.clone())
        };
        self.folders.move_folder(backend, folder.id, target);
    }

    /// Opened, in its own space, which is then the one shown.
    fn choose_folder(&mut self, id: &str) {
        if let Some(space) = self.folders.choose(id) {
            self.spaces.choose_space(Some(space));
        }
    }

    /// The space alone: its row means the space itself, out of every folder.
    fn choose_space(&mut self, id: Option<String>) {
        self.spaces.choose_space(id);
        self.folders.leave();
    }

    /// The switch, remembered for the space shown: there is no board across every space.
    fn set_view(&mut self, mode: board::Mode, backend: &Backend) {
        let Some(space_id) = self.spaces.active_space() else {
            return;
        };
        mode.write(&mut self.library_preferences, space_id);
        drop(api::preferences::write_library(
            backend,
            self.library_preferences.clone(),
        ));
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

    /// While a Note is open: its toolbar offers the library's drawings.
    fn follow_drawings(&mut self, backend: &Backend) {
        let note_open = self
            .editor
            .opened()
            .and_then(|open| open.note())
            .is_some_and(|note| note.kind == engine::notes::kind::NoteKind::Note);
        if !note_open {
            return;
        }
        self.drawing_choices.want(self.revision);
        let answered = self
            .drawing_choices
            .update(|_| api::notes::list_drawings(backend));
        if answered && let Some(error) = self.drawing_choices.error() {
            log::warn!("The drawings were not listed: {error:?}");
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
        self.folders.remember(&mut self.library_preferences);
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
                self.folders.restore(&preferences);
                self.notes_query.restore(&preferences);
                self.http.restore(&preferences);
                self.prettier = crate::notes::prettier::PrettierSettings::read(
                    preferences.get(crate::notes::prettier::KEY),
                );
                self.preview_lines = preferences
                    .get(PREVIEW_LINES_KEY)
                    .and_then(|stored| stored.parse().ok())
                    .unwrap_or_default();
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
            Action::App(AppAction::SetEditorPanelWidth(width)) => {
                self.settings.editor_panel_width = width;
                self.save_settings(backend);
            }
            Action::App(AppAction::SetHistoryWidth(width)) => {
                self.settings.history_list_width = width;
                self.save_settings(backend);
            }
            Action::App(AppAction::SearchTools) => {
                if self.gate.is_open() {
                    self.settings.area = Area::Tools;
                    self.save_settings(backend);
                    self.ask_tool_search();
                }
            }
            Action::Gate(action) => self.gate.apply(action, &mut self.gate_view, backend),
            Action::Notes(action) => self.apply_notes(action, ctx, backend),
            Action::Http(action) => self.apply_http(action, ctx, backend),
            Action::Tools(action) => self.apply_tools(action, backend),
            Action::Menu(action) => self.apply_menu(action, ctx, backend),
            Action::Global(GlobalAction::Quit) => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            // A note opens over the canvas: what the native side asks comes back to the notes.
            Action::Global(GlobalAction::NewNote) => {
                self.apply(Action::App(AppAction::ShowArea(Area::Notes)), ctx, backend);
                self.new_note(None);
            }
            // Not before a library is open: there is nothing to search.
            Action::Global(global @ (GlobalAction::Palette | GlobalAction::PaletteHere)) => {
                if self.gate.is_open() {
                    self.palette.open(backend, global == GlobalAction::Palette);
                }
            }
            Action::Palette(asked) => self.apply_palette(asked, ctx, backend),
            Action::Global(GlobalAction::Capture) => {
                self.apply(Action::App(AppAction::ShowArea(Area::Notes)), ctx, backend);
                self.capture(backend);
            }
        }
    }

    fn apply_menu(
        &mut self,
        action: crate::titlebar::Action,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::titlebar::Action as Menu;
        use engine::transfer::model::ExportScope;
        match action {
            Menu::Import => self.transfer.import(backend, &self.dialog),
            Menu::ExportAll => self.transfer.export(backend, ExportScope::Library),
            Menu::ExportSpace => {
                if let Some(space_id) = self.spaces.active_space().map(str::to_owned) {
                    self.transfer
                        .export(backend, ExportScope::Space { space_id });
                }
            }
            Menu::ExportSelection => {
                let ids = self.checked_on_screen();
                if !ids.is_empty() {
                    self.transfer.export(backend, ExportScope::Notes { ids });
                }
            }
            Menu::CopyMarkdown => {
                let ids = self.checked_on_screen();
                if !ids.is_empty() {
                    self.transfer.share(backend, ids);
                }
            }
            Menu::Prompted => {
                if let Some(answer) = self.titlebar_view.answer.take() {
                    self.transfer.answer(answer, backend);
                }
            }
            Menu::Quit => self.apply(Action::Global(GlobalAction::Quit), ctx, backend),
            Menu::OpenLibraries => {
                self.libraries.load(backend);
                let opener = self.titlebar_view.file_anchor;
                self.dialogs
                    .open(Layer::Settings, crate::titlebar::libraries::id(), opener);
            }
            Menu::Libraries(asked) => self.apply_libraries(asked, ctx, backend),
            Menu::OpenPreferences => {
                self.titlebar_view.open_preferences(&self.settings);
                self.security.load_variables(backend);
                self.backups.load(backend);
                let opener = self.titlebar_view.file_anchor;
                self.dialogs
                    .open(Layer::Settings, crate::titlebar::preferences::id(), opener);
            }
            Menu::Preferences(asked) => self.apply_preferences(asked, ctx, backend),
            Menu::OpenHelp(panel) => {
                if panel == crate::titlebar::about_menu::Panel::WhatsNew {
                    self.help.load(backend);
                }
                self.titlebar_view.show_help(panel);
                let opener = self.titlebar_view.about_anchor();
                self.dialogs
                    .open(Layer::App, crate::titlebar::about_menu::id(), opener);
            }
            Menu::CloseHelp => {
                self.titlebar_view.close_help();
                self.dialogs.close(ctx, crate::titlebar::about_menu::id());
            }
            Menu::OpenLink(url) => self.open_link(&url),
            Menu::Update(asked) => self.apply_update(asked, backend),
        }
    }

    fn apply_update(&mut self, asked: crate::banners::update_prompt::Ask, backend: &Backend) {
        use crate::banners::update_prompt::Ask;
        match asked {
            Ask::Check => self.updates.check(backend, false),
            Ask::Install => {
                self.ask_switch(
                    libraries::Goal::Install,
                    http::leave::Reason::Update,
                    backend,
                );
            }
            Ask::Later { skip } => {
                if self.updates.dismiss(skip, &mut self.settings) {
                    self.save_settings(backend);
                }
            }
        }
    }

    /// Only the theme and the density show before a button: the rest of the application reads
    /// the applied settings, never the draft.
    fn apply_preferences(
        &mut self,
        asked: crate::titlebar::preferences::Ask,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::titlebar::preferences::Ask;
        match asked {
            Ask::Preview => {
                if let Some(look) = self.titlebar_view.previewed() {
                    theme::apply(ctx, look.theme, look.density);
                }
            }
            Ask::Apply => self.adopt_draft(ctx, backend),
            Ask::ApplyAndClose => {
                self.adopt_draft(ctx, backend);
                self.close_preferences(ctx);
            }
            Ask::Discard => {
                self.apply_look(ctx);
                self.close_preferences(ctx);
            }
            Ask::ChangePassphrase => {
                let change = crate::titlebar::preferences::change_id();
                self.dialogs.open(Layer::Passphrase, change, None);
            }
            Ask::TypedCurrent => self.security.clear_refusal(),
            Ask::Restore(id) => self.backups.ask(id),
            Ask::KeepLibrary => self.backups.dismiss(),
            Ask::ConfirmRestore => {
                if let Some(id) = self.backups.pending().map(str::to_owned) {
                    let goal = libraries::Goal::Restore(id);
                    self.ask_switch(goal, http::leave::Reason::Restore, backend);
                }
            }
            Ask::SubmitPassphrase => {
                let (current, next) = self.titlebar_view.take_phrases();
                self.security.change(backend, current, next);
            }
            Ask::KeepPassphrase => {
                self.titlebar_view.take_phrases();
                self.security.clear_refusal();
                self.dialogs
                    .close(ctx, crate::titlebar::preferences::change_id());
            }
        }
    }

    fn adopt_draft(&mut self, ctx: &egui::Context, backend: &Backend) {
        if let Some(settings) = self.titlebar_view.adopt() {
            self.settings = settings;
            self.language = self.settings.locale.language();
            self.apply_look(ctx);
            self.save_settings(backend);
        }
        if let Some(lines) = self.titlebar_view.adopt_preview_lines(self.preview_lines) {
            self.set_preview_lines(lines, backend);
        }
        if let Some(rows) = self
            .titlebar_view
            .adopt_variables(self.security.variables())
        {
            self.security.save_variables(backend, rows);
            self.revision.bump();
        }
    }

    fn close_preferences(&mut self, ctx: &egui::Context) {
        self.titlebar_view.close_preferences();
        self.dialogs.close(ctx, crate::titlebar::preferences::id());
    }

    fn apply_libraries(
        &mut self,
        asked: crate::titlebar::libraries::Ask,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::titlebar::libraries::Ask;
        use libraries::Goal;
        match asked {
            Ask::Close => {
                self.libraries.dismiss_deletion();
                self.dialogs.close(ctx, crate::titlebar::libraries::id());
            }
            Ask::Create(name) => {
                self.ask_switch(Goal::Create(name), http::leave::Reason::Switch, backend);
            }
            Ask::Open(id) => {
                self.ask_switch(Goal::Open(id), http::leave::Reason::Switch, backend);
            }
            Ask::Rename { id, name } => self.libraries.rename(backend, id, name),
            Ask::Delete(id) => self.libraries.ask_to_delete(id),
            Ask::KeepIt => self.libraries.dismiss_deletion(),
            Ask::ConfirmDelete => self.libraries.confirm_deletion(backend),
        }
    }

    /// A switch takes what the HTTP tabs hold: asked before the library changes, not after.
    fn ask_switch(
        &mut self,
        goal: libraries::Goal,
        reason: http::leave::Reason,
        backend: &Backend,
    ) {
        if self.http.ask_leave(reason) {
            self.switch_library(goal, backend);
        } else {
            self.libraries.hold(goal);
        }
    }

    /// Closes the open library: the gate comes back once it is answered, and what belonged to
    /// that library goes with `reset_library_scope`.
    fn switch_library(&mut self, goal: libraries::Goal, backend: &Backend) {
        self.http.leaving(backend);
        match goal {
            libraries::Goal::Open(id) => self.gate.leave_for(api::libraries::open(backend, id)),
            libraries::Goal::Create(name) => {
                self.gate
                    .leave_for(api::libraries::create_and_open(backend, name));
            }
            libraries::Goal::Restore(id) => {
                self.gate.leave_for_copy(api::backups::restore(backend, id));
            }
            libraries::Goal::Install => self.updates.install(backend),
        }
    }

    fn checked_on_screen(&self) -> Vec<String> {
        let order = self.on_screen();
        let order: Vec<&str> = order.iter().map(String::as_str).collect();
        self.selection.checked_in(&order)
    }

    /// What a tool holds of a secret goes with it.
    fn leave_tool(&mut self) {
        if let Some(entry) = self.tools.open_tool() {
            self.tools_view.left(entry);
        }
    }

    fn ask_tool_search(&mut self) {
        self.leave_tool();
        self.tools.home(None);
        self.tools_view.want_search();
    }

    fn apply_tools(&mut self, action: crate::tools::Action, backend: &Backend) {
        use crate::tools::Action as Tools;
        match action {
            Tools::Home(filter) => {
                self.leave_tool();
                self.tools.home(filter);
            }
            Tools::Open(id) => {
                if let Some(entry) = crate::tools::catalogue::find(id) {
                    if self.tools.open_tool().is_some_and(|open| open.id != id) {
                        self.leave_tool();
                    }
                    self.tools.open(entry, self.clock.now());
                    self.save_settings(backend);
                }
            }
            Tools::ToggleFavourite(id) => {
                self.tools.toggle_favourite(id);
                self.save_settings(backend);
            }
            Tools::SearchAsked => self.ask_tool_search(),
            Tools::SetZones(zones) => {
                self.tools.set_zones(zones);
                self.save_settings(backend);
            }
            Tools::SetCsvSeparator(separator) => {
                self.tools.set_csv_separator(separator);
                self.save_settings(backend);
            }
            Tools::SampleLoaded => {
                let said = i18n::Message::new(i18n::keys::tools::SAMPLE_LOADED);
                self.status.report(said);
            }
        }
    }

    fn apply_http(&mut self, action: crate::http::Action, ctx: &egui::Context, backend: &Backend) {
        use crate::http::Action as Http;
        let now = ctx.input(|input| input.time);
        match action {
            Http::Toggle(id) => self.http.toggle(&id),
            Http::Write(write) => self.http.write(backend, write),
            Http::KeyMove { item, name, key } => self.http.key_move(backend, (item, name), key),
            Http::AskDelete(item) => self.http.ask_delete(backend, item),
            Http::CancelDelete => self.http.cancel_delete(),
            Http::OpenRequest(id) => {
                self.http.show_page(backend, http::Page::Requests);
                self.http.tabs_mut().open(backend, id);
            }
            Http::NewRequest(kind) => {
                let name = i18n::translate(self.language, i18n::keys::http::tabs::UNTITLED, &[]);
                self.http.tabs_mut().new_request(kind, name);
            }
            Http::Activate(key) => self.http.tabs_mut().activate(&key),
            Http::AskClose(key) => self.http.ask_close(&key),
            Http::Close(key) => self.http.tabs_mut().close(&key),
            Http::CancelClose => self.http.tabs_mut().cancel_close(),
            Http::Save(key) => self.http.tabs_mut().save(backend, &key),
            Http::SaveAndClose(key) => self.http.tabs_mut().save_and_close(backend, &key),
            Http::Edit { key, edit } => self.http.tabs_mut().edit(&key, |draft| draft.apply(edit)),
            Http::ChooseBody { key, kind } => self.http.tabs_mut().choose_body_kind(&key, kind),
            Http::Rewrite { key, part } => self.http.rewrite(backend, &key, part),
            Http::PickFile { key, target } => {
                self.http.pick_file(backend, &self.dialog, key, target);
            }
            Http::AddExtraction { key, rule } => {
                let variable = rule.variable.clone();
                self.http.add_extraction(&key, rule);
                self.status.report(
                    i18n::Message::new(i18n::keys::http::extract::ADDED).with("variable", variable),
                );
            }
            Http::ChooseEnvironment(id) => self.http.choose_environment(id),
            Http::Environments(asked) => self.http.ask_environments(backend, asked, now),
            Http::Define(name) => self.http.define(backend, name, now),
            Http::SendAnyway(key) => self.http.send_anyway(backend, &key, now),
            Http::SendFile(key, path) => self.http.send_file(backend, &key, path, now),
            Http::StopAsking => self.http.stop_asking(),
            Http::Cookies(asked) => self.http.ask_cookies(backend, asked),
            Http::ToggleConsole => self.http.toggle_console(),
            Http::ClearConsole(key) => self.http.clear_console(&key),
            Http::SaveResponseFile { send_id, name } => {
                self.http
                    .kept_mut()
                    .save_file(backend, &self.dialog, send_id, name);
            }
            Http::AskResponseNote(asked) => {
                self.http.kept_mut().ask_response_note(backend, *asked);
            }
            Http::AskNote(asked) => self.http.kept_mut().ask_note(*asked),
            Http::SaveNote(placement) => self.http.kept_mut().save_note(backend, placement),
            Http::CancelNote => self.http.kept_mut().cancel_note(),
            Http::OpenScript { key, stop } => self.http.open_script(backend, key, &stop),
            Http::OpenLevel { origin, phase } => {
                self.http
                    .open_settings_of(backend, &origin.id, (phase, None));
            }
            Http::ScriptFocusTaken => self.http.script_focus_taken(),
            Http::SendMessage { key, text } => self.http.send_message(backend, &key, text),
            Http::ClearSocketLog(key) => self.http.clear_socket_log(&key),
            Http::Runner(asked) => self.http.ask_runner(backend, &self.dialog, asked),
            Http::Leave(asked) => {
                self.quitting &= asked != http::leave::Asked::Cancel;
                self.http.answer_leave(asked);
            }
            Http::History(asked) => self.http.ask_history(backend, asked),
            Http::Code(asked) => self.http.ask_code(asked),
            Http::CopyCode(text) => self.say_copied(ctx, text, i18n::keys::http::code::COPIED),
            Http::SaveTo { key, target } => self.http.tabs_mut().save_to(backend, &key, target),
            Http::CancelSaveTo => self.http.tabs_mut().cancel_save_to(),
            Http::OpenSettings(container) => self.http.open_settings(backend, container),
            Http::EditSettings(settings) => self.http.edit_settings(*settings),
            Http::SaveSettings => self.http.save_settings(backend),
            Http::CloseSettings => self.http.close_settings(),
            Http::ShowPage(page) => self.http.show_page(backend, page),
            Http::TogglePage(page) => self.http.toggle_page(backend, page),
            Http::Send(key) => self.http.send(backend, key.as_deref(), now),
            Http::CancelSend(key) => self.http.cancel_send(backend, &key),
            Http::Explore { send_id, query } => self.http.explore(&send_id, query),
            Http::ResponseImage(send_id) => self.http.ask_image(backend, &send_id),
            Http::Copy(text) => self.say_copied(ctx, text, i18n::keys::settings::COPIED),
            Http::ClearResponse(key) => self.http.clear_response(backend, &key),
            Http::SetShare(share) => {
                self.settings.http_request_share = share;
                self.save_settings(backend);
            }
            Http::SaveActive => {
                let dirty = self.http.tabs().active().filter(|tab| tab.is_dirty());
                if let Some(key) = dirty.map(|tab| tab.key.clone()) {
                    self.http.tabs_mut().save(backend, &key);
                }
            }
        }
        self.remember_http(backend);
    }

    fn say_copied(&mut self, ctx: &egui::Context, text: String, said: i18n::Key) {
        ctx.copy_text(text);
        self.status.report(i18n::Message::new(said));
    }

    /// The window asks to close: `true` once nothing holds it. The requests are asked about; a
    /// note open closes by its own path, which writes its drafts, and the window follows.
    pub fn may_close(&mut self) -> bool {
        if !self.gate.is_open() {
            return true;
        }
        // Asked before the download: once is enough.
        let answered = self.updates.status() == crate::services::updates::Status::Installed;
        if !answered && !self.http.ask_leave(http::leave::Reason::Quit) {
            return false;
        }
        self.closing_editor = self.editor.is_open();
        !self.closing_editor
    }

    /// The question before leaving is asked over any area, and goes on whichever is shown.
    fn update_area_or_exit(&mut self, ctx: &egui::Context, backend: &Backend, time: f64) {
        if self.settings.area == Area::Http || self.http.leave().is_under_way() {
            self.update_http(ctx, backend, time);
        }
        if self.settings.area == Area::Tools
            && let Some(entry) = self.tools.open_tool()
        {
            let mut turn = crate::tools::live::Turn {
                ctx,
                backend,
                time,
                now: self.clock.now(),
                errors: &mut self.errors,
                status: &mut self.status,
                clipboard: &self.clipboard,
                dialog: &self.dialog,
                zones: self.tools.zones(),
                csv_separator: self.tools.csv_separator(),
            };
            self.tools_view.update(entry, &mut turn);
        }
        if self.settings.area == Area::Tools {
            self.http.update_kept(backend, &mut self.errors);
            self.report_kept();
        }
        if self.closing_editor && !self.editor.is_open() {
            self.closing_editor = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn update_http(&mut self, ctx: &egui::Context, backend: &Backend, time: f64) {
        self.http.update((ctx, backend), time, &mut self.errors);
        let tz_offset = notes_query::tz_offset_minutes(self.clock.now());
        self.http
            .update_history((ctx, backend), (time, tz_offset), &mut self.errors);
        self.report_kept();
        if let Some(count) = self.http.take_history_cleared() {
            let cleared = i18n::Message::new(i18n::keys::http::history::CLEARED);
            self.status.report(cleared.with("count", count));
        }
        if let Some(count) = self.http.take_cookies_removed() {
            let removed = i18n::Message::new(i18n::keys::http::cookies::REMOVED);
            self.status.report(removed.with("count", count));
        }
        self.remember_http(backend);
    }

    /// A note kept, from an answer or from a tool: the canvas learns of it, wherever it is.
    fn report_kept(&mut self) {
        let kept = self.http.take_kept_answer();
        if kept.note_made {
            self.revision.bump();
        }
        if let Some(said) = kept.said {
            self.status.report(said);
        }
    }

    fn remember_http(&mut self, backend: &Backend) {
        if self.http.remember(&mut self.library_preferences) {
            drop(api::preferences::write_library(
                backend,
                self.library_preferences.clone(),
            ));
        }
    }

    /// The spaces and the folders: chosen, made, renamed, deleted.
    fn apply_place(&mut self, action: crate::notes::Action, backend: &Backend) {
        use crate::notes::Action as Notes;
        match action {
            Notes::ChooseSpace(id) => {
                self.choose_space(id);
                self.remember_place(backend);
            }
            Notes::ChooseFolder(id) => {
                self.choose_folder(&id);
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
            } => self.folders.create(backend, space_id, parent_id, &name),
            Notes::RenameFolder { id, name } => self.folders.rename(backend, id, &name),
            Notes::RecolourFolder { id, colour } => self.folders.recolour(backend, id, colour),
            Notes::DeleteFolder(id) => self.folders.delete(backend, id, self.preview_lines),
            Notes::MoveFolder { id, parent, space } => {
                self.move_folder(backend, id, (parent, space));
            }
            Notes::NudgeFolder { id, outwards } => self.nudge_folder(backend, &id, outwards),
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
            Notes::Undo => self.revert_last(backend),
            Notes::DismissUndo => self.undo.dismiss(),
            Notes::DismissFolderName => {
                self.folders.dismiss_naming();
                self.board.dismiss_zone();
            }
            Notes::Tags(asked) => self.tags.apply(asked, backend),
            Notes::Fill(asked) => {
                if asked == crate::notes::Fill::Cancel {
                    self.palette.copied();
                }
                if let Some((text, said)) = self.fill.apply(asked, backend) {
                    self.copy_out(ctx, backend, text, said);
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
            | Notes::DeleteFolder(_)
            | Notes::MoveFolder { .. }
            | Notes::NudgeFolder { .. }) => self.apply_place(place, backend),
            Notes::SetView(mode) => self.set_view(mode, backend),
            Notes::Tidy(scope) => self.tidy(backend, scope),
            Notes::Board(gesture) => self.board_gesture(gesture, ctx, backend),
            Notes::NameZone(name) => {
                if let Some(place) = self.board_place() {
                    self.board.name_zone(backend, &place, &name);
                }
            }
            Notes::Search(typed) => self.notes_query.type_search(ctx, typed),
            Notes::ClearSearch => self.notes_query.clear_search(),
            Notes::SetFilter(filter) => self.notes_query.set_filter(filter),
            Notes::ClearFilters => self.notes_query.clear_filters(),
            Notes::Arrange(arrangement) => {
                self.notes_query.arrange(arrangement);
                arrangement.write(&mut self.library_preferences);
                drop(api::preferences::write_library(
                    backend,
                    self.library_preferences.clone(),
                ));
            }
            Notes::OpenNote(id) => self.open_in_editor(EditorNext::Note(id)),
            Notes::Editor(editing) => self.apply_editing(editing, ctx, backend),
            Notes::NewNote(kind) => self.new_note_asked(kind, backend),
            Notes::Pin { id, pinned } => {
                self.notes.pin(backend, vec![id], pinned);
            }
            Notes::Prioritise { id, priority } => {
                self.notes.prioritise(backend, vec![id], priority);
            }
            Notes::File { id, folder } => {
                self.notes
                    .file(backend, (vec![id], folder), self.preview_lines);
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
                    self.notes
                        .file(backend, (vec![id], folder), self.preview_lines);
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
            Editing::Close => self.close_editor(),
            Editing::Step(by) => self.step_in_editor(by),
            Editing::Present { title, markdown } => {
                self.presentation.ask(backend, title, markdown);
            }
            Editing::StopPresenting => self.presentation.close(ctx),
            Editing::Revisions(asked) => self.apply_revisions(asked, backend),
            Editing::Decimals(places) => self.set_decimals(places, backend),
            display @ (Editing::Fullscreen(_) | Editing::FoldHeader(_) | Editing::Details(_)) => {
                self.set_editor_display(&display, backend);
            }
        }
        ctx.request_repaint();
    }

    /// A display preference of the editor, kept from note to note.
    fn set_editor_display(&mut self, display: &crate::notes::Editing, backend: &Backend) {
        use crate::notes::Editing;
        match *display {
            Editing::Fullscreen(on) => self.settings.editor_fullscreen = on,
            Editing::FoldHeader(folded) => self.settings.editor_header_folded = folded,
            Editing::Details(open) => self.settings.editor_details_open = open,
            _ => return,
        }
        self.save_settings(backend);
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
                    self.copy_out(ctx, backend, text, said);
                }
            }
            Fields::Fold(open) => {
                self.settings.editor_fields_open = open;
                self.save_settings(backend);
            }
        }
    }

    /// The notes on screen, in the order they are drawn: the board's, dimmed ones included,
    /// while it shows — a card the search dims there is still on screen and still selectable.
    pub fn on_screen(&self) -> Vec<String> {
        if self.board_place().is_some() {
            return self
                .board
                .cards()
                .map(|entry| entry.note.id.clone())
                .collect();
        }
        self.notes_query.view().map_or_else(Vec::new, |view| {
            view.cards().map(|card| card.id.clone()).collect()
        })
    }

    fn pinned_on_screen(&self, id: &str) -> Option<bool> {
        if self.board_place().is_some() {
            return self
                .board
                .cards()
                .find(|entry| entry.note.id == id)
                .map(|entry| entry.note.pinned);
        }
        let view = self.notes_query.view()?;
        view.cards()
            .find(|card| card.id == id)
            .map(|card| card.pinned)
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
            Batch::File(folder) => self.notes.file(backend, (ids, folder), self.preview_lines),
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
        let pinned = focused.as_deref().and_then(|id| self.pinned_on_screen(id));
        match key {
            CanvasKey::New(kind) => self.new_note(Some(kind)),
            CanvasKey::Move(direction) => {
                let board = self.board_place().is_some();
                let on_board = self.on_screen();
                let sections: Vec<Vec<&str>> = if board {
                    // The board is walked card by card, zone by zone, as it is laid out.
                    vec![on_board.iter().map(String::as_str).collect()]
                } else if let Some(view) = self.notes_query.view() {
                    view.sections
                        .iter()
                        .map(|section| section.cards.iter().map(|card| card.id.as_str()).collect())
                        .collect()
                } else {
                    return;
                };
                let columns = if board {
                    1
                } else {
                    self.notes_view.canvas.columns
                };
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
            CanvasKey::Undo => self.revert_last(backend),
            CanvasKey::Align => self.tidy(backend, engine::folders::board::BoardScope::LooseCards),
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
            self.revert_last(backend);
            return;
        }
        if self.selection.clear() {
            return;
        }
        if self.notes_query.has_user_filters() {
            self.notes_query.clear_filters();
            return;
        }
        let Some(open) = self.active_folder() else {
            return;
        };
        let parent = self
            .folders
            .find(open)
            .map(|folder| (folder.space_id.clone(), folder.parent_id.clone()));
        match parent {
            Some((_, Some(parent))) => self.choose_folder(&parent),
            Some((space, None)) => self.choose_space(Some(space)),
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
        self.copy_out(ctx, backend, text, placeholder_fill::copied_note(&note));
    }

    /// Onto the clipboard, and said; a copy asked from the palette closes it. Over another
    /// application the window goes back where it came from: a toast is the only way left to say
    /// it.
    pub(crate) fn copy_out(
        &mut self,
        ctx: &egui::Context,
        backend: &Backend,
        text: String,
        said: i18n::Message,
    ) {
        ctx.copy_text(text);
        let from_palette = self.palette.copied();
        let from_elsewhere = from_palette && self.palette.from_elsewhere();
        if from_palette {
            self.palette.close();
        }
        if from_elsewhere {
            put_away(ctx);
        }
        if !self.settings.copy_confirmation {
            return;
        }
        if from_elsewhere {
            self.say_on_the_desktop(backend, &said);
        } else {
            self.status.report(said);
        }
    }

    fn say_on_the_desktop(&self, backend: &Backend, said: &i18n::Message) {
        let title = i18n::translate(self.language, i18n::keys::palette::COPIED_TITLE, &[]);
        let body = said.text(self.language);
        let notifier = std::sync::Arc::clone(&self.notifier);
        drop(backend.compute(move |_| {
            notifier.notify(&title, &body);
            Ok(())
        }));
    }

    fn apply_palette(
        &mut self,
        asked: crate::palette::Asked,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::palette::Asked;
        use crate::palette::commands::Command;
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
                    Chosen::Command(Command::Area(area)) => Some(area),
                    Chosen::Command(Command::Request) | Chosen::Request(_) => Some(Area::Http),
                    Chosen::Tool(_) => Some(Area::Tools),
                    // Over whichever area is on screen.
                    Chosen::Command(Command::Preferences | Command::Shortcuts) => None,
                    _ => Some(Area::Notes),
                };
                if let Some(area) = area {
                    self.apply(Action::App(AppAction::ShowArea(area)), ctx, backend);
                }
                self.run_chosen(chosen, ctx, backend);
            }
        }
    }

    fn run_chosen(
        &mut self,
        chosen: crate::palette::options::Chosen,
        ctx: &egui::Context,
        backend: &Backend,
    ) {
        use crate::palette::commands::Command;
        use crate::palette::options::Chosen;
        match chosen {
            Chosen::Note(id) => self.open_in_editor(EditorNext::Note(id)),
            Chosen::Request(id) => {
                let open = crate::http::Action::OpenRequest(id);
                self.apply(Action::Http(open), ctx, backend);
            }
            Chosen::Tool(id) => {
                let open = crate::tools::Action::Open(id);
                self.apply(Action::Tools(open), ctx, backend);
            }
            Chosen::Command(Command::Request) => {
                let draft = crate::http::Action::NewRequest(engine::http::model::RequestKind::Http);
                self.apply(Action::Http(draft), ctx, backend);
            }
            Chosen::Command(Command::Preferences) => {
                let open = crate::titlebar::Action::OpenPreferences;
                self.apply(Action::Menu(open), ctx, backend);
            }
            Chosen::Command(Command::Shortcuts) => {
                let panel = crate::titlebar::about_menu::Panel::Sheet;
                let open = crate::titlebar::Action::OpenHelp(panel);
                self.apply(Action::Menu(open), ctx, backend);
            }
            Chosen::Space(id) => {
                self.choose_space(Some(id));
                self.remember_place(backend);
            }
            Chosen::Command(Command::Create(kind)) => self.new_note(Some(kind)),
            Chosen::Command(Command::Folder) => self.folders.ask_name(self.spaces.active_space()),
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

    /// How the sheets on screen write their results.
    pub fn calc_format(&self) -> crate::notes::calc::Format {
        crate::notes::calc::Format::of(&self.settings, self.language)
    }

    /// A modal over the canvas takes its keys: the canvas's stand aside. The editor is one for
    /// the keys, though it sits beside the cards.
    pub fn modal_open(&self) -> bool {
        self.editor.is_open() || self.overlay_open()
    }

    /// What opens over the canvas and the editor beside it.
    pub fn overlay_open(&self) -> bool {
        self.trash.is_open()
            || self.tags.is_open()
            || self.fill.is_open()
            || self.palette.is_open()
            || self.folders.naming()
            || self.board.drawn_zone().is_some()
            || self.http.leave().is_asking()
            || self.dialogs.top().is_some()
    }

    /// In the space shown, or the first one for « all spaces », and the folder shown.
    fn set_decimals(&mut self, places: Option<u8>, backend: &Backend) {
        self.settings.calc_decimals = places;
        self.save_settings(backend);
    }

    /// From the menu: a Note made once, its entry stops saying « New ».
    fn new_note_asked(&mut self, kind: Option<engine::notes::kind::NoteKind>, backend: &Backend) {
        self.new_note(kind);
        if kind == Some(engine::notes::kind::NoteKind::Note) && !self.settings.note_kind_tried {
            self.settings.note_kind_tried = true;
            self.save_settings(backend);
        }
    }

    fn new_note(&mut self, kind: Option<engine::notes::kind::NoteKind>) {
        self.open_in_editor(EditorNext::Draft(kind));
    }

    /// Now if nothing is open; otherwise once the note open has gone by its closing path, which
    /// writes what was typed in it.
    fn open_in_editor(&mut self, next: EditorNext) {
        if !self.editor.is_open() {
            self.open_now(next);
            return;
        }
        if let EditorNext::Note(id) | EditorNext::Stepped(id) = &next
            && self.editor.open_id() == Some(id.as_str())
        {
            return;
        }
        self.editor_next = Some(next);
    }

    fn open_now(&mut self, next: EditorNext) {
        match next {
            EditorNext::Note(id) => self.editor.open(id),
            EditorNext::Stepped(id) => self.editor.step_to(id),
            EditorNext::Draft(kind) => {
                let Some((space, folder)) = self.new_place() else {
                    return;
                };
                let kind = kind.unwrap_or(self.settings.default_note_kind);
                self.editor.draft(kind, space, folder);
            }
        }
    }

    /// Then what was asked while it was open, if anything.
    fn close_editor(&mut self) {
        self.editor.close();
        if let Some(next) = self.editor_next.take() {
            self.open_now(next);
        }
    }

    /// J and K: the note before or after the one open, in the order the cards are shown.
    fn step_in_editor(&mut self, by: isize) {
        let Some((at, order)) = self.editor_position() else {
            return;
        };
        let Some(next) = at.checked_add_signed(by).and_then(|to| order.get(to)) else {
            return;
        };
        self.open_in_editor(EditorNext::Stepped(next.clone()));
    }

    /// Where the note open sits among the cards shown, and those cards; `None` for a draft or a
    /// note the view does not show.
    pub fn editor_position(&self) -> Option<(usize, Vec<String>)> {
        let id = self.editor.open_id()?;
        let order = self.on_screen();
        let at = order.iter().position(|shown| shown == id)?;
        Some((at, order))
    }

    /// The clipboard read like a paste into an empty Note: code makes a snippet in the language
    /// the core reads in it, prose a Note. Holding its content, it is written at once.
    fn capture(&mut self, backend: &Backend) {
        use engine::notes::kind::NoteKind;
        use engine::notes::language::Language;
        if !self.gate.is_open() {
            return;
        }
        let Some(content) = self.clipboard.text().filter(|text| !text.trim().is_empty()) else {
            return;
        };
        let Some((space, folder)) = self.new_place() else {
            return;
        };
        let language = crate::api::notes::detect_language(&content);
        let kind = if language == Language::Txt {
            NoteKind::Note
        } else {
            NoteKind::Snippet
        };
        self.editor
            .draft_filled(backend, (kind, space, folder), content, language);
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
        Some((space, self.active_folder().map(str::to_owned)))
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

    /// The desktop's own browser, for the web alone.
    fn open_link(&self, url: &str) {
        platform::links::open(self.opener.as_ref(), url);
    }

    /// The library's, like the Prettier settings: the board's loose cards are stored there in
    /// pixels, so their height must be the library's too.
    fn set_preview_lines(&mut self, lines: PreviewLines, backend: &Backend) {
        self.preview_lines = lines;
        self.library_preferences
            .set(PREVIEW_LINES_KEY, lines.as_str());
        drop(api::preferences::write_library(
            backend,
            self.library_preferences.clone(),
        ));
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
                    let indent = self.settings.code_indent.unit(note.language);
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
        self.tools.write(&mut self.preferences);
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
        self.preview_lines = PreviewLines::default();
        // A library left for the gate has nothing to read until it is unlocked.
        self.library_preferences_read = self
            .gate
            .is_open()
            .then(|| api::preferences::library(backend));
        self.seeding = None;
        self.presentation.close(ctx);
        self.caches = LibraryCaches::default();
        self.notes_query = NotesQueryState::default();
        self.spaces = spaces::SpacesState::default();
        self.folders = folders::FoldersState::default();
        self.board = board::BoardState::default();
        self.editor = editor::EditorState::default();
        self.revisions = note_revisions::RevisionsState::default();
        self.drawing_choices = QueryCell::default();
        self.selection = selection::SelectionState::default();
        self.undo = undo::UndoState::default();
        self.trash = trash::TrashState::default();
        self.tags = tags::TagsState::default();
        self.fill = placeholder_fill::FillState::default();
        self.palette = palette::PaletteState::default();
        self.notes_view = crate::notes::View::default();
        self.http = http::HttpState::default();
        self.http_view = crate::http::View::default();
        self.tools = tools::ToolsState::read(&self.preferences);
        self.tools_view = crate::tools::View::default();
        self.transfer = transfer::TransferState::default();
        self.libraries = libraries::LibrariesState::default();
        self.security = security::SecurityState::default();
        self.backups = backups::BackupsState::default();
        self.titlebar_view = crate::titlebar::View::default();
        self.dialogs = crate::widgets::DialogStack::default();
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
        state.gate.ask(&backend);
        backend.settle();
        state.update(&ctx, &backend);
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

    /// The library's, not the application's: read when it opens, written into its own file.
    #[test]
    fn the_preview_lines_are_the_librarys() {
        let (backend, _profile) = testing::unlocked();
        let directory =
            engine::db::run_here(backend.engine(), engine::libraries::open_library_directory)
                .unwrap();
        let file = directory.join(engine::layout::PREFERENCES);
        let mut library = Preferences::read(&file);
        library.set(PREVIEW_LINES_KEY, "5");
        library.write(&file).unwrap();
        let ctx = egui::Context::default();
        let mut state = AppState::new(Preferences::default(), true);
        state.gate.ask(&backend);
        backend.settle();
        state.update(&ctx, &backend);
        state.reset_library_scope(&ctx, &backend);
        backend.settle();
        state.update(&ctx, &backend);
        assert_eq!(state.preview_lines, PreviewLines::Five);

        state.set_preview_lines(PreviewLines::Three, &backend);
        backend.settle();

        assert_eq!(Preferences::read(&file).get(PREVIEW_LINES_KEY), Some("3"));
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
