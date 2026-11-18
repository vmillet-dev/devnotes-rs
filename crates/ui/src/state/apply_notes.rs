//! What the notes ask: the canvas and the board, folders, the editor and its overlays.

use super::{
    AppState, EditorNext, PREVIEW_LINES_KEY, attachments, board, drawing_pictures, editor,
    placeholder_fill, put_away,
};
use crate::api;
use crate::backend::Backend;
use engine::folders::board::PreviewLines;

impl AppState {
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
    pub(super) fn board_gesture(
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
    pub(super) fn tidy(&mut self, backend: &Backend, scope: engine::folders::board::BoardScope) {
        if let Some(place) = self.board_place() {
            self.board
                .arrange(backend, &place, scope, self.preview_lines);
        }
    }

    /// The last write put back; an arrangement taken back pans the ground back to where it was.
    pub(super) fn revert_last(&mut self, backend: &Backend) {
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
    pub(super) fn move_folder(
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
    pub(super) fn nudge_folder(&mut self, backend: &Backend, id: &str, outwards: bool) {
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
    pub(super) fn choose_folder(&mut self, id: &str) {
        if let Some(space) = self.folders.choose(id) {
            self.spaces.choose_space(Some(space));
        }
    }

    /// The space alone: its row means the space itself, out of every folder.
    pub(super) fn choose_space(&mut self, id: Option<String>) {
        self.spaces.choose_space(id);
        self.folders.leave();
    }

    /// The switch, remembered for the space shown: there is no board across every space.
    pub(super) fn set_view(&mut self, mode: board::Mode, backend: &Backend) {
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
    pub(super) fn follow_revisions(&mut self, backend: &Backend) {
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
    pub(super) fn follow_drawings(&mut self, backend: &Backend) {
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
    pub(super) fn follow_in_editor(&mut self, done: &api::undo::Reversible) {
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
    pub(super) fn remember_place(&mut self, backend: &Backend) {
        self.spaces.remember(&mut self.library_preferences);
        self.folders.remember(&mut self.library_preferences);
        drop(api::preferences::write_library(
            backend,
            self.library_preferences.clone(),
        ));
    }

    /// The spaces and the folders: chosen, made, renamed, deleted.
    pub(super) fn apply_place(&mut self, action: crate::notes::Action, backend: &Backend) {
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
    pub(super) fn apply_overlay(
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

    pub(super) fn apply_notes(
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
    pub(super) fn apply_editing(
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
    pub(super) fn set_editor_display(
        &mut self,
        display: &crate::notes::Editing,
        backend: &Backend,
    ) {
        use crate::notes::Editing;
        match *display {
            Editing::Fullscreen(on) => self.settings.editor_fullscreen = on,
            Editing::FoldHeader(folded) => self.settings.editor_header_folded = folded,
            Editing::Details(open) => self.settings.editor_details_open = open,
            _ => return,
        }
        self.save_settings(backend);
    }

    pub(super) fn ask_attachment(&mut self, asked: attachments::Asked, backend: &Backend) {
        let moment = self.clock.now().format("capture-%Y-%m-%d-%H-%M-%S");
        self.attachments
            .ask(asked, backend, &self.dialog, moment.to_string());
    }

    pub(super) fn apply_fields(
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

    pub(super) fn pinned_on_screen(&self, id: &str) -> Option<bool> {
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
    pub(super) fn batch(&mut self, batch: crate::notes::Batch, backend: &Backend) {
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
    pub(super) fn canvas_key(
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
    pub(super) fn step_back(&mut self, ctx: &egui::Context, backend: &Backend) {
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
    pub(super) fn copied(
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

    /// How the sheets on screen write their results.
    pub fn calc_format(&self) -> crate::notes::calc::Format {
        crate::notes::calc::Format::of(&self.settings, self.language)
    }

    /// In the space shown, or the first one for « all spaces », and the folder shown.
    pub(super) fn set_decimals(&mut self, places: Option<u8>, backend: &Backend) {
        self.settings.calc_decimals = places;
        self.save_settings(backend);
    }

    /// From the menu: a Note made once, its entry stops saying « New ».
    pub(super) fn new_note_asked(
        &mut self,
        kind: Option<engine::notes::kind::NoteKind>,
        backend: &Backend,
    ) {
        self.new_note(kind);
        if kind == Some(engine::notes::kind::NoteKind::Note) && !self.settings.note_kind_tried {
            self.settings.note_kind_tried = true;
            self.save_settings(backend);
        }
    }

    pub(super) fn new_note(&mut self, kind: Option<engine::notes::kind::NoteKind>) {
        self.open_in_editor(EditorNext::Draft(kind));
    }

    /// Now if nothing is open; otherwise once the note open has gone by its closing path, which
    /// writes what was typed in it.
    pub(super) fn open_in_editor(&mut self, next: EditorNext) {
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

    pub(super) fn open_now(&mut self, next: EditorNext) {
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
    pub(super) fn close_editor(&mut self) {
        self.editor.close();
        if let Some(next) = self.editor_next.take() {
            self.open_now(next);
        }
    }

    /// J and K: the note before or after the one open, in the order the cards are shown.
    pub(super) fn step_in_editor(&mut self, by: isize) {
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
    pub(super) fn capture(&mut self, backend: &Backend) {
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
    pub(super) fn new_place(&mut self) -> Option<(String, Option<String>)> {
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
    pub(super) fn apply_revisions(&mut self, asked: crate::notes::Revisions, backend: &Backend) {
        use crate::notes::Revisions;
        match asked {
            Revisions::Toggle => self.revisions.toggle(),
            Revisions::Look(id) => self.revisions.look(backend, &id),
            Revisions::Back => self.revisions.close_preview(),
            Revisions::Restore => self.revisions.restore(backend),
        }
    }

    /// The library's, like the Prettier settings: the board's loose cards are stored there in
    /// pixels, so their height must be the library's too.
    pub(super) fn set_preview_lines(&mut self, lines: PreviewLines, backend: &Backend) {
        self.preview_lines = lines;
        self.library_preferences
            .set(PREVIEW_LINES_KEY, lines.as_str());
        drop(api::preferences::write_library(
            backend,
            self.library_preferences.clone(),
        ));
    }

    pub(super) fn set_prettier(
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
    pub(super) fn save_formatted(
        &mut self,
        backend: &Backend,
        session: u64,
        mut edit: editor::Edit,
    ) {
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
}
