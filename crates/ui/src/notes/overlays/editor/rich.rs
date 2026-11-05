//! A Note edited formatted (the reference's `rich-text-editor`): the toolbar, then the document
//! drawn by `widgets::rich` with the caret and the selection crossing its blocks. Every change
//! goes through `richtext::edit` and its undo; the Markdown is written back only once the user
//! changed something, so a Note opened and left keeps its stored body byte for byte.

use egui::text::CCursor;
use egui::{Event, Key, Modifiers, Pos2, Rect, Sense, Stroke, Ui, UiBuilder, pos2, vec2};
use i18n::{Language, keys};
use richtext::edit::{self, ordered};
use richtext::inlines::{self, Mark};
use richtext::{Document, History, ListKind, Marks, Position, Selection, tree};

use crate::state::AppState;
use crate::theme::palette;
use crate::widgets;
use crate::widgets::rich::{Embedded, Laid, RichView, Sketch};

const TOOLBAR_HEIGHT: f32 = 40.0;
const PADDING: egui::Vec2 = egui::vec2(32.0, 20.0);

/// What the last change was: typing in a row is undone as one step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Typing,
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
    /// The caret's x, kept through ↑ and ↓.
    goal_x: Option<f32>,
    link: Option<LinkForm>,
    reveal: bool,
    /// The text takes the keys back after a tool: the caret stays where it was.
    refocus: bool,
    search: super::search_bar::SearchBar,
    /// When the text last changed, in the input's seconds: a diagram redraws once typing pauses.
    changed_at: f64,
}

/// Typing redraws a diagram once the keys pause; the drawing before stays meanwhile.
const REDRAW_AFTER_SECS: f64 = 0.3;

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
            goal_x: None,
            link: None,
            reveal: false,
            refocus: false,
            search: super::search_bar::SearchBar::default(),
            changed_at: 0.0,
        }
    }

    /// The Markdown to write, once the user changed the Note; `None` before.
    pub(super) fn markdown(&self) -> Option<String> {
        self.edited.then(|| richtext::write(&self.document))
    }

    /// Before a change: undo comes back here.
    fn record(&mut self, step: Step) {
        if !(step == Step::Typing && self.last == Some(Step::Typing)) {
            self.history.record(&self.document, &self.selection);
        }
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
        let leaves = tree::leaves(&self.document);
        let index = leaves.iter().position(|path| *path == head.path);
        let next = match (forward, index) {
            (true, _) if head.offset < length => Some(Position {
                offset: head.offset + 1,
                ..head
            }),
            (false, _) if head.offset > 0 => Some(Position {
                offset: head.offset - 1,
                ..head
            }),
            (true, Some(at)) => leaves.get(at + 1).map(|path| Position {
                path: path.clone(),
                offset: 0,
            }),
            (false, Some(at)) => at.checked_sub(1).map(|at| Position {
                path: leaves[at].clone(),
                offset: tree::leaf(&self.document, &leaves[at]).map_or(0, |leaf| leaf.len()),
            }),
            _ => None,
        };
        if let Some(next) = next {
            self.move_head(next, extend);
        }
        self.goal_x = None;
    }

    fn move_head(&mut self, head: Position, extend: bool) {
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

    /// The place under a point: the leaf whose lines it is on, or the nearest one.
    fn position_at(&self, point: Pos2) -> Option<Position> {
        let laid = self
            .laid
            .iter()
            .min_by(|a, b| distance(a, point).total_cmp(&distance(b, point)))?;
        let local = point - laid.origin;
        let offset = laid.galley.cursor_from_pos(local).index.0;
        Some(Position {
            path: laid.path.clone(),
            offset,
        })
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
    /// The diagrams to draw, by their source.
    pub(super) diagrams: Vec<String>,
}

pub(super) fn show(ui: &mut Ui, view: &mut RichEditor, state: &AppState) -> Shown {
    let language = state.language;
    let whole = ui.max_rect();
    let mut shown = Shown::default();
    let before = view.changes;
    let bar = Rect::from_min_max(whole.min, pos2(whole.right(), whole.top() + TOOLBAR_HEIGHT));
    let mut bar_ui = ui.new_child(UiBuilder::new().id_salt("rich-toolbar").max_rect(bar));
    toolbar::show(&mut bar_ui, view, language);
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
    widgets::address(ui, &surface, "editor-rich");
    describe(ui, &surface, view, language);
    let reveal = std::mem::take(&mut view.reveal);
    let caret = view.caret_rect();
    let found = search::lit(view, &view.search);
    let pictures = |id: &str| embedded(state, id);
    let missing = i18n::translate(language, keys::drawing::MISSING, &[]);
    let ctx = ui.ctx().clone();
    let diagrams = |source: &str| state.caches.diagrams.get(&ctx, source);
    let scrolled = egui::ScrollArea::vertical()
        .id_salt("rich-scroll")
        .auto_shrink([false, false])
        .show(&mut body_ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(32, 20))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let painted = RichView::new(&view.document)
                        .editing(&view.selection)
                        .found(&found)
                        .pictures(&pictures, &missing)
                        .diagrams(&diagrams, language)
                        .show(ui);
                    if reveal && let Some(caret) = caret {
                        ui.scroll_to_rect(caret.expand(PADDING.y), None);
                    }
                    painted
                })
                .inner
        });
    let painted = scrolled.inner;
    view.laid = painted.leaves;
    shown.wanted = painted.wanted;
    shown.diagrams = to_draw(ui, view, painted.sketches);
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
    if let Some(code) = painted.diagram {
        to_code(view, code, &surface);
    }
    pointer(ui, view, &surface);
    if focused {
        lock_keys(ui, surface.id);
        shown.code_paste = keys(ui, view, state.clipboard.as_ref());
        paint_caret(ui, view, body);
    }
    shown.left = surface.lost_focus();
    shown.changed = view.changes != before;
    if shown.changed {
        view.changed_at = ui.input(|input| input.time);
    }
    ui.advance_cursor_after_rect(whole);
    shown
}

/// The diagrams to ask for: at once when nothing stands in their place, once typing pauses
/// otherwise.
fn to_draw(ui: &Ui, view: &RichEditor, sketches: Vec<Sketch>) -> Vec<String> {
    let quiet = ui.input(|input| input.time) - view.changed_at;
    let mut asked = Vec::new();
    for sketch in sketches {
        if !sketch.stale || quiet >= REDRAW_AFTER_SECS {
            asked.push(sketch.source);
        } else {
            let left = std::time::Duration::from_secs_f64(REDRAW_AFTER_SECS - quiet);
            ui.ctx().request_repaint_after(left);
        }
    }
    asked
}

/// A diagram clicked: the caret at the end of its code, which unfolds.
fn to_code(view: &mut RichEditor, code: Vec<usize>, surface: &egui::Response) {
    let end = tree::leaf(&view.document, &code).map_or(0, |leaf| leaf.len());
    view.move_head(
        Position {
            path: code,
            offset: end,
        },
        false,
    );
    surface.request_focus();
}

/// What stands where the Note embeds drawing `id`: its large picture, read by id once shown.
pub(in crate::notes) fn embedded(state: &AppState, id: &str) -> Option<Embedded> {
    use crate::state::drawing_pictures::{Thumbnail, Thumbnails};
    match state.caches.pictures.get(&Thumbnails::embedded(id)) {
        Thumbnail::Unknown => None,
        Thumbnail::Waiting => Some(Embedded::Waiting),
        Thumbnail::Missing => Some(Embedded::Missing),
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

/// A press places the caret, a drag extends it, Shift extends from where it is.
/// The keys stay on the surface.
fn pointer(ui: &Ui, view: &mut RichEditor, surface: &egui::Response) {
    let Some(at) = surface.interact_pointer_pos() else {
        return;
    };
    let shift = ui.input(|input| input.modifiers.shift);
    if (surface.drag_started() || surface.clicked())
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
            } => key_pressed(view, key, physical_key, modifiers),
            _ => {}
        }
    }
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

fn copy(ui: &Ui, view: &RichEditor) {
    if !view.selection.is_empty() {
        ui.ctx()
            .copy_text(edit::selected_text(&view.document, &view.selection));
    }
}

fn key_pressed(view: &mut RichEditor, key: Key, physical: Option<Key>, modifiers: Modifiers) {
    if let Some(command) = shortcuts::find(key, physical, modifiers) {
        run_command(view, command);
        return;
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
}

fn run_command(view: &mut RichEditor, command: shortcuts::Command) {
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

mod search;
mod shortcuts;
mod toolbar;

#[cfg(test)]
mod tests;
