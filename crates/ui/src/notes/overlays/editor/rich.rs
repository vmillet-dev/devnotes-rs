//! A Note edited formatted (the reference's `rich-text-editor`): the toolbar, then the document
//! drawn by `widgets::rich` with the caret and the selection crossing its blocks. Every change
//! goes through `richtext::edit` and its undo; the Markdown is written back only once the user
//! changed something, so a Note opened and left keeps its stored body byte for byte.

use crate::theme::tokens::gap;
use egui::text::CCursor;
use egui::{Event, Key, Modifiers, Pos2, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};
use i18n::{Language, keys};
use richtext::edit::{self, ordered};
use richtext::inlines::{self, Mark};
use richtext::picture::{self, Beside};
use richtext::{Document, History, ListKind, Marks, Position, Selection, tree};

use crate::state::AppState;
use crate::theme::palette;
use crate::widgets;
use crate::widgets::rich::{Embedded, Laid, LaidImage, RichView};

const TOOLBAR_HEIGHT: f32 = 40.0;
const PADDING: egui::Vec2 = egui::vec2(gap::PAGE, gap::PANEL);

/// What the last change was: typing in a row is undone as one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Typing,
    /// A drawing made larger or smaller, from where it was taken to where it is let go.
    Sizing,
    Other,
}

/// The link being written, from the toolbar.
#[derive(Default)]
struct LinkForm {
    text: String,
    address: String,
}

pub(super) struct RichEditor {
    document: Document,
    selection: Selection,
    history: History,
    last: Option<Step>,
    edited: bool,
    /// Counts the changes: a frame that moved it changed the text.
    changes: u64,
    /// What the next typed text wears, chosen on a caret.
    stored: Option<Marks>,
    /// Where each leaf was laid out last frame.
    laid: Vec<Laid>,
    images: Vec<LaidImage>,
    /// The image the keys are for, in the caret's place.
    picked: Option<Vec<usize>>,
    /// The caret's x, kept through ↑ and ↓.
    goal_x: Option<f32>,
    link: Option<LinkForm>,
    reveal: bool,
    /// The text takes the keys back after a tool: the caret stays where it was.
    refocus: bool,
    search: super::search_bar::SearchBar,
}

impl RichEditor {
    pub(super) fn of(markdown: &str) -> Self {
        let mut document = richtext::read(markdown);
        // A new Note is empty: it still has a line to type on.
        if document.blocks.is_empty() {
            document.blocks.push(richtext::Block::Paragraph(Vec::new()));
        }
        let first = tree::leaves(&document)
            .into_iter()
            .next()
            .unwrap_or_default();
        Self {
            document,
            selection: Selection::caret(Position {
                path: first,
                offset: 0,
            }),
            history: History::default(),
            last: None,
            edited: false,
            changes: 0,
            stored: None,
            laid: Vec::new(),
            images: Vec::new(),
            picked: None,
            goal_x: None,
            link: None,
            reveal: false,
            refocus: false,
            search: super::search_bar::SearchBar::default(),
        }
    }

    /// The Markdown to write, once the user changed the Note; `None` before.
    pub(super) fn markdown(&self) -> Option<String> {
        self.edited.then(|| richtext::write(&self.document))
    }

    /// Before a change: undo comes back here.
    fn record(&mut self, step: Step) {
        let same = step != Step::Other && self.last == Some(step);
        if !same {
            self.history.record(&self.document, &self.selection);
        }
        self.picked = None;
        self.last = Some(step);
        self.edited = true;
        self.changes += 1;
        self.goal_x = None;
        self.reveal = true;
    }

    fn change(&mut self, step: Step, change: impl FnOnce(&mut Document, &Selection) -> Selection) {
        self.record(step);
        let selection = change(&mut self.document, &self.selection);
        self.selection = Selection::caret(tree::clamp(&self.document, &selection.head));
        if selection.anchor != selection.head {
            self.selection = selection;
        }
    }

    /// Text typed, then the input rule it closes, undone on its own.
    fn type_text(&mut self, text: &str) {
        let marks = self.stored.take().unwrap_or_else(|| self.marks_here());
        self.change(Step::Typing, |document, selection| {
            edit::insert_text(document, selection, text, &marks)
        });
        let before = self.document.clone();
        let Some(applied) = richtext::rules::after_typing(&mut self.document, &self.selection)
        else {
            return;
        };
        let typed = std::mem::replace(&mut self.document, before);
        self.record(Step::Other);
        self.document = typed;
        self.selection = applied.selection;
        if let Some(mark) = applied.mark {
            let mut next = marks;
            inlines::put(&mut next, &mark, false);
            self.stored = Some(next);
        }
    }

    /// The marks of the `char` before the caret.
    fn marks_here(&self) -> Marks {
        let head = &self.selection.head;
        match tree::leaf(&self.document, &head.path) {
            Some(tree::Leaf::Inlines(content)) => inlines::marks_at(content, head.offset),
            _ => Marks::default(),
        }
    }

    fn toggle(&mut self, mark: &Mark) {
        if self.selection.is_empty() {
            let mut marks = self.stored.take().unwrap_or_else(|| self.marks_here());
            let on = match mark {
                Mark::Bold => !marks.bold,
                Mark::Italic => !marks.italic,
                Mark::Strike => !marks.strike,
                Mark::Code => !marks.code,
                Mark::Link(_) => true,
            };
            inlines::put(&mut marks, mark, on);
            self.stored = Some(marks);
            return;
        }
        self.record(Step::Other);
        edit::toggle_mark(&mut self.document, &self.selection, mark);
    }

    fn undo(&mut self, redo: bool) {
        let now = (&self.document, &self.selection);
        let back = if redo {
            self.history.redo(now)
        } else {
            self.history.undo(now)
        };
        if let Some((document, selection)) = back {
            self.document = document;
            self.selection = selection;
            self.picked = None;
            self.last = None;
            self.edited = true;
            self.changes += 1;
            self.reveal = true;
        }
    }

    fn select_all(&mut self) {
        let leaves = tree::leaves(&self.document);
        let (Some(first), Some(last)) = (leaves.first(), leaves.last()) else {
            return;
        };
        let end = tree::leaf(&self.document, last).map_or(0, |leaf| leaf.len());
        self.selection = Selection {
            anchor: Position {
                path: first.clone(),
                offset: 0,
            },
            head: Position {
                path: last.clone(),
                offset: end,
            },
        };
    }

    /// The caret one `char` along, across leaves; `extend` keeps the anchor.
    fn step(&mut self, forward: bool, extend: bool) {
        let head = self.selection.head.clone();
        if !extend && !self.selection.is_empty() {
            let (start, end) = ordered(&self.selection);
            self.selection = Selection::caret(if forward { end } else { start });
            return;
        }
        let length = tree::leaf(&self.document, &head.path).map_or(0, |leaf| leaf.len());
        let inside = if forward {
            (head.offset < length).then_some(head.offset + 1)
        } else {
            head.offset.checked_sub(1)
        };
        if let Some(offset) = inside {
            self.move_head(Position { offset, ..head }, extend);
        } else {
            self.step_past(head.path, forward, extend);
        }
        self.goal_x = None;
    }

    /// Off a leaf or an image: onto the image beside it, which a selection goes over.
    fn step_past(&mut self, mut path: Vec<usize>, forward: bool, extend: bool) {
        loop {
            match picture::beside(&self.document, &path, forward) {
                Some(Beside::Image(image)) if extend => path = image,
                Some(Beside::Image(image)) => return self.pick(image),
                Some(Beside::Leaf(position)) => return self.move_head(position, extend),
                None => return,
            }
        }
    }

    /// The word holding the `char` at `at`, as the code editor reads a word; `extend`, the
    /// selection stretched to it.
    fn select_word(&mut self, at: Position, extend: bool) {
        let text = match tree::leaf(&self.document, &at.path) {
            Some(tree::Leaf::Inlines(content)) => inlines::plain(content),
            Some(tree::Leaf::Code(text)) => text.clone(),
            None => return,
        };
        let word = text::word_around(&text, at.offset);
        let start = Position {
            path: at.path.clone(),
            offset: word.start,
        };
        let end = Position {
            path: at.path,
            offset: word.end,
        };
        if extend {
            let head = if start >= self.selection.anchor {
                end
            } else {
                start
            };
            self.move_head(head, true);
        } else {
            self.move_head(start, false);
            self.move_head(end, true);
        }
        self.goal_x = None;
    }

    /// The whole paragraph, heading, item's line, cell or code block at `path`.
    fn select_leaf(&mut self, path: Vec<usize>) {
        let Some(len) = tree::leaf(&self.document, &path).map(|leaf| leaf.len()) else {
            return;
        };
        self.move_head(
            Position {
                path: path.clone(),
                offset: 0,
            },
            false,
        );
        self.move_head(Position { path, offset: len }, true);
        self.goal_x = None;
    }

    fn move_head(&mut self, head: Position, extend: bool) {
        self.picked = None;
        if extend {
            self.selection.head = head;
        } else {
            self.selection = Selection::caret(head);
        }
        self.reveal = true;
    }

    /// Nothing written yet: one empty paragraph.
    fn is_empty(&self) -> bool {
        matches!(self.document.blocks.as_slice(), [richtext::Block::Paragraph(content)] if content.is_empty())
    }

    /// Where a leaf laid out last frame puts the caret, on screen.
    fn caret_rect(&self) -> Option<Rect> {
        let head = &self.selection.head;
        let laid = self.laid.iter().find(|laid| laid.path == head.path)?;
        let local = laid.galley.pos_from_cursor(CCursor::new(head.offset));
        Some(local.translate(laid.origin.to_vec2()))
    }

    /// The leaf whose lines a point is on, or the nearest one.
    fn laid_at(&self, point: Pos2) -> Option<&Laid> {
        self.laid
            .iter()
            .min_by(|a, b| distance(a, point).total_cmp(&distance(b, point)))
    }

    /// The place under a point: the boundary between two `char`s nearest it.
    fn position_at(&self, point: Pos2) -> Option<Position> {
        let laid = self.laid_at(point)?;
        let offset = laid.galley.cursor_from_pos(point - laid.origin).index.0;
        Some(Position {
            path: laid.path.clone(),
            offset,
        })
    }

    /// The `char` under a point: before the nearest boundary when the point is left of it.
    fn char_at(&self, point: Pos2) -> Option<Position> {
        let laid = self.laid_at(point)?;
        let local = point - laid.origin;
        let cursor = laid.galley.cursor_from_pos(local);
        let before = local.x < laid.galley.pos_from_cursor(cursor).center().x;
        Some(Position {
            path: laid.path.clone(),
            offset: cursor.index.0.saturating_sub(usize::from(before)),
        })
    }

    /// ↓ with nothing under a code block: a paragraph is opened there.
    fn leave_code(&mut self) {
        let path = &self.selection.head.path;
        if matches!(tree::leaf(&self.document, path), Some(tree::Leaf::Code(_))) {
            self.change(Step::Other, |document, selection| {
                edit::leave_code(document, selection).unwrap_or_else(|| selection.clone())
            });
        }
    }

    /// ↑ and ↓: a line up or down at the same x, into the leaf above or below.
    fn vertical(&mut self, down: bool, extend: bool) {
        let Some(caret) = self.caret_rect() else {
            return;
        };
        let x = *self.goal_x.get_or_insert(caret.left());
        let y = if down {
            caret.bottom() + 2.0
        } else {
            caret.top() - 2.0
        };
        let inside = self
            .laid
            .iter()
            .find(|laid| laid_rect(laid).y_range().contains(y));
        let target = inside.or_else(|| {
            if down {
                self.laid
                    .iter()
                    .filter(|laid| laid_rect(laid).top() > caret.bottom())
                    .min_by(|a, b| laid_rect(a).top().total_cmp(&laid_rect(b).top()))
            } else {
                self.laid
                    .iter()
                    .filter(|laid| laid_rect(laid).bottom() < caret.top())
                    .max_by(|a, b| laid_rect(a).bottom().total_cmp(&laid_rect(b).bottom()))
            }
        });
        let Some(target) = target else {
            if down && !extend {
                self.leave_code();
            }
            return;
        };
        let rect = laid_rect(target);
        let y = y.clamp(rect.top() + 1.0, rect.bottom() - 1.0);
        let offset = target
            .galley
            .cursor_from_pos(pos2(x, y) - target.origin)
            .index
            .0;
        let head = Position {
            path: target.path.clone(),
            offset,
        };
        let goal = self.goal_x;
        self.move_head(head, extend);
        self.goal_x = goal;
    }
}

fn laid_rect(laid: &Laid) -> Rect {
    laid.galley.rect.translate(laid.origin.to_vec2())
}

fn distance(laid: &Laid, point: Pos2) -> f32 {
    let rect = laid_rect(laid).expand2(vec2(0.0, 2.0));
    if rect.y_range().contains(point.y) {
        (point.x - rect.x_range().clamp(point.x)).abs() * 0.001
    } else {
        (point.y - rect.center().y).abs()
    }
}

/// What a frame of the Note did.
#[derive(Default)]
pub(super) struct Shown {
    pub(super) changed: bool,
    pub(super) left: bool,
    /// A link followed with Ctrl+click: `http` and `https` alone.
    pub(super) link: Option<String>,
    /// Code pasted into the empty Note: it becomes a snippet in the language detected.
    pub(super) code_paste: Option<String>,
    /// The embedded drawings whose picture is to be read.
    pub(super) wanted: Vec<String>,
}

pub(super) fn show(ui: &mut Ui, view: &mut RichEditor, state: &AppState) -> Shown {
    let language = state.language;
    let whole = ui.max_rect();
    let mut shown = Shown::default();
    let before = view.changes;
    let bar = Rect::from_min_max(whole.min, pos2(whole.right(), whole.top() + TOOLBAR_HEIGHT));
    let mut bar_ui = ui.new_child(UiBuilder::new().id_salt("rich-toolbar").max_rect(bar));
    let drawings = state.drawing_choices.value().map_or(&[][..], Vec::as_slice);
    toolbar::show(&mut bar_ui, view, drawings, language);
    let mut top = bar.bottom();
    if view.link.is_some() {
        let form = Rect::from_min_max(pos2(whole.left(), top), pos2(whole.right(), top + 40.0));
        let mut form_ui = ui.new_child(UiBuilder::new().id_salt("rich-link").max_rect(form));
        toolbar::link_form(&mut form_ui, view, language);
        top = form.bottom();
    }
    if view.search.open {
        let room = Rect::from_min_max(pos2(whole.left(), top), whole.max);
        let mut bar_ui = ui.new_child(UiBuilder::new().id_salt("rich-search").max_rect(room));
        let mut bar = std::mem::take(&mut view.search);
        if super::search_bar::show(&mut bar_ui, &mut bar, view, language).is_some() {
            view.changes += 1;
        }
        view.search = bar;
        top = bar_ui.min_rect().bottom();
    }
    let p = palette(ui.ctx());
    ui.painter()
        .hline(whole.x_range(), top, Stroke::new(1.0, p.line_soft));
    let body = Rect::from_min_max(pos2(whole.left(), top + 1.0), whole.max);
    let mut body_ui = ui.new_child(UiBuilder::new().id_salt("rich-body").max_rect(body));
    let surface = widgets::region(&mut body_ui, body, "editor-rich", Sense::click_and_drag());
    // Before the body is drawn: a link under the pointer says it is one.
    if surface.hovered() || surface.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
    }
    widgets::address(ui, &surface, "editor-rich");
    describe(ui, &surface, view, language);
    let reveal = std::mem::take(&mut view.reveal);
    let caret = view.caret_rect();
    let found = search::lit(view, &view.search);
    let pictures = |id: &str| embedded(state, id);
    let missing = i18n::translate(language, keys::drawing::MISSING, &[]);
    let scrolled = egui::ScrollArea::vertical()
        .id_salt("rich-scroll")
        .auto_shrink([false, false])
        .show(&mut body_ui, |ui| {
            egui::Frame::new()
                .inner_margin(gap::margin(gap::PAGE, gap::PANEL))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let painted = RichView::new(&view.document)
                        .editing(&view.selection)
                        .found(&found)
                        .pictures(&pictures, &missing)
                        .show(ui);
                    if reveal && let Some(caret) = caret {
                        ui.scroll_to_rect(caret.expand(PADDING.y), None);
                    }
                    images::controls(ui, view, &painted.images, language);
                    painted
                })
                .inner
        });
    let painted = scrolled.inner;
    view.laid = painted.leaves;
    view.images = painted.images;
    if let Some(image) = painted.picked {
        view.pick(image);
        view.refocus = true;
    }
    shown.wanted = painted.wanted;
    if let Some(link) = painted.link {
        shown.link = Some(link);
    }
    if std::mem::take(&mut view.refocus) {
        surface.request_focus();
    }
    let focused = widgets::takes_keys(ui, &surface);
    if focused && let Some(replacing) = super::search_bar::shortcut(ui) {
        let mut bar = std::mem::take(&mut view.search);
        bar.open(view, replacing);
        view.search = bar;
    }
    if let Some((list, item)) = painted.ticked {
        view.record(Step::Other);
        edit::toggle_task(&mut view.document, &list, item);
        surface.request_focus();
    }
    pointer(ui, view, &surface);
    menu(view, &surface, state.language);
    if focused {
        lock_keys(ui, surface.id);
        shown.code_paste = keys(ui, view, state.clipboard.as_ref());
        if view.picked.is_none() {
            paint_caret(ui, view, body);
        }
    }
    shown.left = surface.lost_focus();
    shown.changed = view.changes != before;
    ui.advance_cursor_after_rect(whole);
    shown
}

/// What stands where the Note embeds drawing `id`: its large picture, read by id once shown.
pub(in crate::notes) fn embedded(state: &AppState, id: &str) -> Option<Embedded> {
    use crate::state::drawing_pictures::{Thumbnail, Thumbnails};
    match state.caches.pictures.get(&Thumbnails::embedded(id)) {
        Thumbnail::Unknown => None,
        Thumbnail::Waiting => Some(Embedded::Waiting),
        Thumbnail::Empty | Thumbnail::Missing => Some(Embedded::Missing),
        Thumbnail::Shown(texture) => Some(Embedded::Shown(texture.clone())),
    }
}

/// Tab and the arrows are the text's while it has the keys; Escape is not, it closes.
fn lock_keys(ui: &Ui, id: egui::Id) {
    let filter = egui::EventFilter {
        tab: true,
        horizontal_arrows: true,
        vertical_arrows: true,
        escape: false,
    };
    ui.memory_mut(|memory| memory.set_focus_lock_filter(id, filter));
}

/// The surface's node: a text field holding the Note's words, for kittest and a reader.
fn describe(ui: &Ui, surface: &egui::Response, view: &RichEditor, language: Language) {
    let label = i18n::translate(language, keys::editor::CONTENT_LABEL, &[]);
    let value: String = tree::leaves(&view.document)
        .iter()
        .filter_map(|path| match tree::leaf(&view.document, path) {
            Some(tree::Leaf::Inlines(content)) => Some(inlines::plain(content)),
            Some(tree::Leaf::Code(text)) => Some(text.clone()),
            None => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    surface.widget_info(|| egui::WidgetInfo::labeled(egui::Role::MultilineTextInput, true, &label));
    ui.ctx().accesskit_node_builder(surface.id, |node| {
        node.set_role(egui::accesskit::Role::MultilineTextInput);
        node.set_label(label.clone());
        node.set_value(value);
    });
}

/// A press places the caret, a drag extends it, Shift extends from where it is; a double click
/// takes the word, a triple the paragraph. The keys stay on the surface.
fn pointer(ui: &Ui, view: &mut RichEditor, surface: &egui::Response) {
    let Some(at) = surface.interact_pointer_pos() else {
        return;
    };
    let shift = ui.input(|input| input.modifiers.shift);
    // egui reports the click of a double click too, and both of a triple: the longest first.
    if surface.triple_clicked()
        && let Some(position) = view.position_at(at)
    {
        view.select_leaf(position.path);
        surface.request_focus();
    } else if surface.double_clicked()
        && let Some(position) = view.char_at(at)
    {
        view.select_word(position, shift);
        surface.request_focus();
    } else if (surface.drag_started() || surface.clicked())
        && let Some(position) = view.position_at(at)
    {
        view.move_head(position, shift);
        view.goal_x = None;
        surface.request_focus();
    } else if surface.dragged()
        && let Some(position) = view.position_at(at)
    {
        view.move_head(position, true);
    }
}

/// The frame's keys and clipboard events; code pasted into the empty Note is answered.
fn keys(
    ui: &Ui,
    view: &mut RichEditor,
    clipboard: &dyn platform::clipboard::Clipboard,
) -> Option<String> {
    let events = ui.input(|input| input.events.clone());
    let mut code_paste = None;
    // A shortcut the text ran is taken from the frame, or the area's keys would run it too:
    // `Ctrl+B` is bold here and the rail's elsewhere (D14).
    let mut ran = Vec::new();
    for event in events {
        match event {
            Event::Text(text) | Event::Ime(egui::ImeEvent::Commit(text)) => view.type_text(&text),
            Event::Paste(text) => code_paste = code_paste.or(paste(view, &text, clipboard)),
            Event::Copy => copy(ui, view),
            Event::Cut => {
                copy(ui, view);
                view.change(Step::Other, edit::delete_selection);
            }
            Event::Key {
                key,
                physical_key,
                pressed: true,
                modifiers,
                ..
            } => {
                let shortcut = key_pressed(ui, view, key, physical_key, modifiers);
                if shortcut {
                    ran.push((modifiers, key));
                }
            }
            _ => {}
        }
    }
    ui.input_mut(|input| {
        for (modifiers, key) in ran {
            input.consume_key(modifiers, key);
        }
    });
    code_paste
}

/// A page's formatting kept, code into the empty Note answered, anything else as text.
fn paste(
    view: &mut RichEditor,
    text: &str,
    clipboard: &dyn platform::clipboard::Clipboard,
) -> Option<String> {
    if let Some(html) = clipboard.html().filter(|html| richtext::formatted(html)) {
        let pasted = richtext::from_html(&html);
        view.change(Step::Other, |document, selection| {
            edit::insert_document(document, selection, pasted)
        });
        return None;
    }
    let text = text.replace("\r\n", "\n");
    let code = !text.trim().is_empty()
        && view.is_empty()
        && crate::api::notes::detect_language(&text) != engine::notes::language::Language::Txt;
    if code {
        return Some(text);
    }
    view.change(Step::Other, |document, selection| {
        edit::insert_text(document, selection, &text, &Marks::default())
    });
    None
}

/// The selection as text, its lists' markers kept.
fn copy(ui: &Ui, view: &RichEditor) {
    if !view.selection.is_empty() {
        ui.ctx()
            .copy_text(edit::copied(&view.document, &view.selection));
    }
}

fn copy_markdown(ui: &Ui, view: &RichEditor) {
    if !view.selection.is_empty() {
        ui.ctx()
            .copy_text(edit::copied_markdown(&view.document, &view.selection));
    }
}

/// Cut, copy, copy as Markdown, paste, select all. Paste asks the window for the clipboard: it
/// arrives at the next frame as an event the keys take.
fn menu(view: &mut RichEditor, surface: &egui::Response, language: Language) {
    use i18n::keys::editor::menu as words;
    let popup = egui::Popup::context_menu(surface).frame(crate::widgets::menu_frame(&surface.ctx));
    popup.show(|ui| {
        crate::widgets::fitted_menu(ui, 0.0, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let entries = [
                (words::CUT, "rich-menu-cut"),
                (words::COPY, "rich-menu-copy"),
                (words::COPY_MARKDOWN, "rich-menu-copy-markdown"),
                (words::PASTE, "rich-menu-paste"),
                (words::SELECT_ALL, "rich-menu-select-all"),
            ];
            for (key, author_id) in entries {
                let label = i18n::translate(language, key, &[]);
                if !crate::widgets::menu_entry(ui, &label, author_id, false, false, false).clicked()
                {
                    continue;
                }
                match author_id {
                    "rich-menu-cut" if !view.selection.is_empty() => {
                        copy(ui, view);
                        view.change(Step::Other, edit::delete_selection);
                    }
                    "rich-menu-copy" => copy(ui, view),
                    "rich-menu-copy-markdown" => copy_markdown(ui, view),
                    "rich-menu-paste" => ui
                        .ctx()
                        .send_viewport_cmd(egui::ViewportCommand::RequestPaste),
                    "rich-menu-select-all" => view.select_all(),
                    _ => {}
                }
                ui.close();
                surface.request_focus();
            }
        });
    });
}

/// `true` when the key was one of the text's shortcuts.
fn key_pressed(
    ui: &Ui,
    view: &mut RichEditor,
    key: Key,
    physical: Option<Key>,
    modifiers: Modifiers,
) -> bool {
    if let Some(command) = shortcuts::find(key, physical, modifiers) {
        run_command(ui, view, command);
        return true;
    }
    if let Some(shortcut) = images::key_pressed(view, key, modifiers) {
        return shortcut;
    }
    match key {
        Key::Enter => view.change(Step::Other, edit::split),
        Key::Backspace => view.change(Step::Other, edit::delete_backward),
        Key::Delete => view.change(Step::Other, edit::delete_forward),
        Key::ArrowLeft => view.step(false, modifiers.shift),
        Key::ArrowRight => view.step(true, modifiers.shift),
        Key::ArrowUp => view.vertical(false, modifiers.shift),
        Key::ArrowDown => view.vertical(true, modifiers.shift),
        Key::Home | Key::End => line_edge(view, key == Key::End, modifiers),
        Key::Tab => tab(view, modifiers.shift),
        _ => {}
    }
    false
}

fn run_command(ui: &Ui, view: &mut RichEditor, command: shortcuts::Command) {
    use shortcuts::Command;
    use toolbar::Tool;
    match command {
        Command::Bold => view.toggle(&Mark::Bold),
        Command::Italic => view.toggle(&Mark::Italic),
        Command::Strike => view.toggle(&Mark::Strike),
        Command::Code => view.toggle(&Mark::Code),
        Command::Paragraph => {
            view.record(Step::Other);
            edit::set_heading(&mut view.document, &view.selection, None);
        }
        Command::Heading(level) => {
            view.record(Step::Other);
            edit::set_heading(&mut view.document, &view.selection, Some(level));
        }
        Command::Bullet => toolbar::run(view, Tool::List(ListKind::Bullet)),
        Command::Ordered => toolbar::run(view, Tool::List(ListKind::Ordered { start: 1 })),
        Command::Tasks => toolbar::run(view, Tool::List(ListKind::Task)),
        Command::Quote => toolbar::run(view, Tool::Quote),
        Command::CodeBlock => view.change(Step::Other, edit::toggle_code_block),
        Command::HardBreak => view.change(Step::Other, edit::hard_break),
        Command::Undo => view.undo(false),
        Command::Redo => view.undo(true),
        Command::SelectAll => view.select_all(),
        Command::CopyMarkdown => copy_markdown(ui, view),
    }
}

/// Home and End: the line's edges; with Ctrl, the document's.
fn line_edge(view: &mut RichEditor, end: bool, modifiers: Modifiers) {
    if modifiers.command {
        let leaves = tree::leaves(&view.document);
        let target = if end { leaves.last() } else { leaves.first() };
        if let Some(path) = target {
            let offset = if end {
                tree::leaf(&view.document, path).map_or(0, |leaf| leaf.len())
            } else {
                0
            };
            view.move_head(
                Position {
                    path: path.clone(),
                    offset,
                },
                modifiers.shift,
            );
        }
        return;
    }
    let head = view.selection.head.clone();
    let Some(laid) = view.laid.iter().find(|laid| laid.path == head.path) else {
        return;
    };
    let caret = laid.galley.pos_from_cursor(CCursor::new(head.offset));
    let x = if end {
        f32::INFINITY
    } else {
        f32::NEG_INFINITY
    };
    let offset = laid
        .galley
        .cursor_from_pos(egui::vec2(x, caret.center().y))
        .index
        .0;
    view.move_head(Position { offset, ..head }, modifiers.shift);
}

/// Tab walks a table's cells, indents code, nests an item under the one above; Shift+Tab goes
/// back. Elsewhere a tab is text.
fn tab(view: &mut RichEditor, back: bool) {
    let path = view.selection.head.path.clone();
    if richtext::table::cell_of(&view.document, &path).is_some() {
        view.selection = richtext::table::step(&mut view.document, &view.selection, back);
        view.reveal = true;
        return;
    }
    if let Some(tree::Leaf::Code(_)) = tree::leaf(&view.document, &path) {
        let unit = code_indent(view);
        if back {
            view.change(Step::Other, |document, selection| {
                edit::outdent_code(document, selection, &unit)
            });
        } else {
            view.type_text(&unit);
        }
        return;
    }
    let item = tree::innermost_item(&view.document, &path);
    match (item, back) {
        (Some(_), false) => view.change(Step::Other, edit::sink_item),
        (Some((list, item, _)), true) => {
            view.change(Step::Other, |document, _| {
                edit::lift_item(document, &list, item)
            });
        }
        (None, false) => view.type_text("\t"),
        (None, true) => untab(view),
    }
}

/// One level of indentation in the code block the caret is in, by its language's habit.
fn code_indent(view: &RichEditor) -> String {
    let language = match tree::block(&view.document, &view.selection.head.path) {
        Some(richtext::Block::CodeBlock { language, .. }) => language.parse().ok(),
        _ => None,
    };
    language
        .map_or("  ", crate::notes::language::indent_unit)
        .to_owned()
}

/// Shift+Tab outside a list: the tab a line starts with goes.
fn untab(view: &mut RichEditor) {
    let path = view.selection.head.path.clone();
    let starts = matches!(
        tree::leaf(&view.document, &path),
        Some(tree::Leaf::Inlines(content)) if inlines::plain(content).starts_with('\t')
    );
    if !starts {
        return;
    }
    let tab = Selection {
        anchor: Position {
            path: path.clone(),
            offset: 0,
        },
        head: Position { path, offset: 1 },
    };
    let offset = view.selection.head.offset.saturating_sub(1);
    view.change(Step::Other, |document, _| {
        edit::delete_selection(document, &tab)
    });
    view.selection.head.offset = offset;
    view.selection.anchor = view.selection.head.clone();
}

fn paint_caret(ui: &Ui, view: &RichEditor, body: Rect) {
    let Some(caret) = view.caret_rect() else {
        return;
    };
    if !body.intersects(caret) {
        return;
    }
    let p = palette(ui.ctx());
    ui.painter().with_clip_rect(body).vline(
        caret.left(),
        caret.y_range(),
        Stroke::new(1.5, p.text_0),
    );
    let to_global = ui
        .ctx()
        .layer_transform_to_global(ui.layer_id())
        .unwrap_or_default();
    ui.ctx().output_mut(|output| {
        output.ime = Some(egui::output::IMEOutput {
            purpose: egui::IMEPurpose::Normal,
            rect: to_global * body,
            cursor_rect: to_global * caret,
            should_interrupt_composition: false,
        });
    });
}

mod images;
mod search;
mod shortcuts;
mod toolbar;

#[cfg(test)]
mod tests;
