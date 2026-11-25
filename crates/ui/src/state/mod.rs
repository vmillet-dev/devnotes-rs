//! What the interface remembers between frames, apart from the drawing.

pub mod attachments;
pub mod backups;
pub mod board;
pub mod calc_sheets;
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

mod apply_http;
mod apply_notes;
mod apply_tools;
mod apply_window;
pub use query::QueryCell;

use engine::folders::board::PreviewLines;

use crate::action::{Action, AppAction, GlobalAction};
use crate::api;
use crate::backend::{Backend, Pending};
use crate::gate::{Gate, GateView};
use crate::notes::destinations::Destination;
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

/// Every cache of a library's content — laid-out text, textures, memos — and only they: one
/// field, emptied when another library opens (D17). Not wiped byte by byte; the threat model
/// says so.
pub struct LibraryCaches {
    pub thumbnails: drawing_pictures::Thumbnails,
    /// The large pictures Notes embed.
    pub pictures: drawing_pictures::Thumbnails,
    /// Where notes were last moved, the latest first.
    pub recent_moves: Vec<Destination>,
}

impl LibraryCaches {
    pub(super) fn moved_to(&mut self, to: Destination) {
        const KEPT: usize = 3;
        self.recent_moves.retain(|recent| *recent != to);
        self.recent_moves.insert(0, to);
        self.recent_moves.truncate(KEPT);
    }
}

impl Default for LibraryCaches {
    fn default() -> Self {
        Self {
            thumbnails: drawing_pictures::Thumbnails::default(),
            pictures: drawing_pictures::Thumbnails::large(),
            recent_moves: Vec::new(),
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

/// Out of sight, for the tray, a key or the Dock to call back: minimised where nothing hides it.
pub(crate) fn put_away(ctx: &egui::Context) {
    ctx.send_viewport_cmd(if platform::window::can_hide() {
        egui::ViewportCommand::Visible(false)
    } else {
        egui::ViewportCommand::Minimized(true)
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
            opener: std::sync::Arc::new(platform::desktop::NoOpener),
            clipboard: std::sync::Arc::new(platform::clipboard::NoClipboard),
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
        self.follow_modals(ctx);
    }

    /// The modals whose state lives in the screens', in the stack while they show: the keys of
    /// what is under them stand aside, and the focus goes back to what opened them.
    fn follow_modals(&mut self, ctx: &egui::Context) {
        let http = &self.http;
        let modals = [
            (Layer::App, "trash-panel", self.trash.is_open()),
            (Layer::App, "tag-manager", self.tags.is_open()),
            (Layer::App, "folder-name-prompt", self.folders.naming()),
            (Layer::Fields, "placeholder-form", self.fill.is_open()),
            (Layer::Palette, "palette", self.palette.is_open()),
            (
                Layer::Zoom,
                "image-lightbox",
                self.attachments.zoomed().is_some(),
            ),
            (
                Layer::Presentation,
                "presentation",
                self.presentation.shown().is_some(),
            ),
            (
                Layer::App,
                "http-close-dialog",
                http.tabs().closing().is_some(),
            ),
            (Layer::App, "http-code-dialog", http.code().key().is_some()),
            (
                Layer::App,
                "http-settings-dialog",
                http.container().is_some(),
            ),
            (Layer::App, "http-cookies-dialog", http.cookies().is_open()),
            (Layer::App, "http-leave-dialog", http.leave().is_asking()),
            (Layer::App, "save-as-note", http.kept().asked().is_some()),
            (
                Layer::App,
                "http-save-dialog",
                http.tabs().asking_place().is_some(),
            ),
            (Layer::App, "note-picker", self.tools_view.is_picking()),
        ];
        for (layer, name, open) in modals {
            self.dialogs
                .follow(ctx, layer, egui::Id::unique(name), open);
        }
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
            Action::App(AppAction::DismissError(said)) => self.errors.dismiss(said),
            Action::App(AppAction::DismissStatus(said)) => self.status.dismiss(said),
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

    fn checked_on_screen(&self) -> Vec<String> {
        let order = self.on_screen();
        let order: Vec<&str> = order.iter().map(String::as_str).collect();
        self.selection.checked_in(&order)
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

    fn say_on_the_desktop(&self, backend: &Backend, said: &i18n::Message) {
        let title = i18n::translate(self.language, i18n::keys::palette::COPIED_TITLE, &[]);
        let body = said.text(self.language);
        let notifier = std::sync::Arc::clone(&self.notifier);
        drop(backend.compute(move |_| {
            notifier.notify(&title, &body);
            Ok(())
        }));
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

    /// The desktop's own browser, for the web alone.
    fn open_link(&self, url: &str) {
        platform::links::open(self.opener.as_ref(), url);
    }

    fn save_settings(&mut self, backend: &Backend) {
        self.settings.write(&mut self.preferences);
        self.tools.write(&mut self.preferences);
        // A write that fails says so in the log (`api::preferences`); nothing on screen waits for it.
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
