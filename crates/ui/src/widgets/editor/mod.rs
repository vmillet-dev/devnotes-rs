//! The code editor's widget (plan § 3.8): `text::Buffer` drawn line by line, only the lines on
//! screen laid out, with a gutter, the hanging indent of wrapped lines that egui's `TextEdit`
//! cannot do, decorations given by the caller, the IME, Tab kept in the text, and the context
//! menu. It knows no rule of the notes: colours, field ranges and marked lines are arguments.

mod bubbles;
mod geometry;
mod input;

use std::ops::Range;

use egui::text::CCursor;
use egui::{
    Color32, FontFamily, FontId, Galley, Id, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
    WidgetInfo,
};
use text::{Buffer, Selection};

use crate::theme::palette;

pub use bubbles::Completion;
pub use geometry::Lines;

/// How far the text sits from the gutter, and the gutter from its numbers.
const PADDING: f32 = 10.0;
/// Past this many characters, the node's value is not the text: an accessibility tree is not
/// where a 1 MB snippet is read.
const VALUE_UP_TO: usize = 64 * 1024;

/// What the caller decorates, as `char` ranges of the whole text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoration {
    pub range: Range<usize>,
    pub mark: Mark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// A `{{field}}`: amber on a faint amber.
    Field,
    /// What the search finds.
    Match,
    /// The match the selection is on.
    CurrentMatch,
}

/// A line's colours as the caller reads them: `char` ranges within the line.
pub type Colours<'a> = &'a dyn Fn(usize, &str) -> Vec<(Range<usize>, Color32)>;

/// What lives between frames: the text, its selection and history, and where things are.
#[derive(Debug, Default)]
pub struct Editor {
    pub buffer: Buffer,
    pub(crate) lines: Lines,
    /// The caret's x, kept through ↑ and ↓ across shorter lines.
    goal_x: Option<f32>,
    /// What the input method is composing, drawn at the caret until it commits.
    preedit: String,
    /// The caret is brought into view at the next frame.
    reveal: bool,
    choosing: bubbles::Choosing,
    /// The text takes the focus at the next frame.
    wants_focus: bool,
    /// The lines the last frame drew.
    visible: Range<usize>,
}

impl Editor {
    pub fn new(text: &str) -> Self {
        Self {
            buffer: Buffer::new(text),
            ..Self::default()
        }
    }

    pub fn text(&self) -> String {
        self.buffer.text()
    }

    /// From outside — a restored revision, a format — as one step of undo.
    pub fn replace_all(&mut self, text: &str) {
        let edit = text::rewrite(&self.buffer.text(), text, self.buffer.selection().head);
        let edited = self.buffer.apply(&edit);
        self.lines.forget_from(edited.first_line);
    }

    /// One step back, as Ctrl+Z takes it: the line the text changed from.
    pub fn undo(&mut self) -> Option<usize> {
        let edited = self.buffer.undo()?;
        self.lines.forget_from(edited.first_line);
        self.reveal = true;
        Some(edited.first_line)
    }

    /// The lines the last frame drew; a first screenful before any.
    pub fn visible_lines(&self) -> Range<usize> {
        if self.visible.is_empty() {
            0..self.buffer.line_count().min(80)
        } else {
            self.visible.clone()
        }
    }

    /// The selection brought into view at the next frame.
    pub fn reveal_selection(&mut self) {
        self.reveal = true;
    }

    /// The text takes the keys at the next frame.
    pub fn focus(&mut self) {
        self.wants_focus = true;
    }

    /// `range` selected and shown.
    pub fn select(&mut self, range: Range<usize>) {
        self.buffer.select(Selection {
            anchor: range.start,
            head: range.end,
        });
        self.reveal = true;
    }
}

/// A line on screen and its first row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
    pub line: usize,
    pub rect: Rect,
}

/// What a frame did.
pub struct Output {
    pub response: Response,
    /// The text changed, from this line on.
    pub changed: Option<usize>,
    /// The lines on screen, for what the caller draws beside them.
    pub rows: Vec<Row>,
}

pub struct EditorView<'a> {
    editor: &'a mut Editor,
    margin: f32,
    author_id: &'a str,
    label: &'a str,
    hint: &'a str,
    unit: &'a str,
    comment: Option<&'a str>,
    wrap: bool,
    gutter: bool,
    read_only: bool,
    font: FontId,
    decorations: &'a [Decoration],
    marked_lines: &'a [usize],
    lit_lines: Range<usize>,
    colours: Option<Colours<'a>>,
    language: i18n::Language,
    completion: Option<Completion<'a>>,
    hover: Option<&'a dyn Fn(usize) -> Option<String>>,
}

impl<'a> EditorView<'a> {
    pub fn new(editor: &'a mut Editor, label: &'a str, author_id: &'a str) -> Self {
        Self {
            editor,
            margin: 0.0,
            author_id,
            label,
            hint: "",
            unit: "  ",
            comment: None,
            wrap: false,
            gutter: true,
            read_only: false,
            font: FontId::new(13.0, FontFamily::Monospace),
            decorations: &[],
            marked_lines: &[],
            lit_lines: 0..0,
            colours: None,
            language: i18n::Language::En,
            completion: None,
            hover: None,
        }
    }

    /// What the caller proposes for the word at the caret (`Editor::word_at_caret`).
    #[must_use]
    pub fn completion(mut self, completion: Option<Completion<'a>>) -> Self {
        self.completion = completion;
        self
    }

    /// The words for a position under the pointer, shown as a tooltip.
    #[must_use]
    pub fn hover(mut self, hover: &'a dyn Fn(usize) -> Option<String>) -> Self {
        self.hover = Some(hover);
        self
    }

    /// The context menu's.
    #[must_use]
    pub fn language(mut self, language: i18n::Language) -> Self {
        self.language = language;
        self
    }

    #[must_use]
    pub fn hint(mut self, hint: &'a str) -> Self {
        self.hint = hint;
        self
    }

    /// One level of indentation: what Tab inserts.
    #[must_use]
    pub fn indent_unit(mut self, unit: &'a str) -> Self {
        self.unit = unit;
        self
    }

    /// The token Ctrl+/ puts before a line; none, and the key does nothing.
    #[must_use]
    pub fn comment(mut self, token: Option<&'a str>) -> Self {
        self.comment = token;
        self
    }

    #[must_use]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    #[must_use]
    pub fn gutter(mut self, gutter: bool) -> Self {
        self.gutter = gutter;
        self
    }

    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[must_use]
    pub fn font(mut self, font: FontId) -> Self {
        self.font = font;
        self
    }

    #[must_use]
    pub fn decorations(mut self, decorations: &'a [Decoration]) -> Self {
        self.decorations = decorations;
        self
    }

    /// Lines marked in the gutter, zero-based and sorted: what a format changed.
    #[must_use]
    pub fn marked_lines(mut self, lines: &'a [usize]) -> Self {
        self.marked_lines = lines;
        self
    }

    /// Lines lit from outside, zero-based: what the explorer beside the code has selected.
    #[must_use]
    pub fn lit_lines(mut self, lines: Range<usize>) -> Self {
        self.lit_lines = lines;
        self
    }

    #[must_use]
    pub fn colours(mut self, colours: Colours<'a>) -> Self {
        self.colours = Some(colours);
        self
    }

    /// Room kept on the right of the text, for what the caller draws there.
    #[must_use]
    pub fn margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Output {
        let id = Id::unique(self.author_id).with(ui.scope_id());
        let row_height = ui.fonts_mut(|fonts| fonts.row_height(&self.font));
        let char_width = ui.fonts_mut(|fonts| fonts.glyph_width(&self.font, '0'));
        let digits = self.editor.buffer.line_count().to_string().len().max(2);
        #[expect(clippy::cast_precision_loss, reason = "a handful of digits")]
        let gutter = if self.gutter {
            digits as f32 * char_width + 2.0 * PADDING
        } else {
            0.0
        };
        let scroll = if self.wrap {
            egui::ScrollArea::vertical()
        } else {
            egui::ScrollArea::both()
        };
        let mut changed = None;
        let mut rows = Vec::new();
        let shown = scroll
            .id_salt(id.with("scroll"))
            .auto_shrink(false)
            .show_viewport(ui, |ui, viewport| {
                self.body(
                    ui,
                    id,
                    (viewport, gutter, row_height, char_width),
                    (&mut changed, &mut rows),
                )
            });
        Output {
            response: shown.inner,
            changed,
            rows,
        }
    }

    fn body(
        self,
        ui: &mut Ui,
        text_id: Id,
        (viewport, gutter, row_height, char_width): (Rect, f32, f32, f32),
        (changed, rows): (&mut Option<usize>, &mut Vec<Row>),
    ) -> Response {
        let origin = ui.max_rect().min;
        let room = ui.available_width() - gutter - 2.0 * PADDING - self.margin;
        let text_width = room.max(char_width * 8.0);
        let frame = geometry::Frame {
            font: self.font.clone(),
            row_height,
            char_width,
            wrap: self.wrap.then_some(text_width),
            text_left: origin.x + gutter + PADDING,
            top: origin.y,
        };
        let editor = self.editor;
        editor.lines.measure(&editor.buffer, &frame);
        let size = Vec2::new(
            gutter + 2.0 * PADDING + editor.lines.widest(&editor.buffer, &frame),
            editor.lines.total() + row_height,
        )
        .max(ui.available_size());
        let rect = Rect::from_min_size(origin, size);
        ui.allocate_rect(rect, Sense::hover());
        // The part on screen only: what is clicked, and what the accessibility tree points at.
        let visible = viewport.translate(origin.to_vec2());
        // ⚠️ Of its own, not the next automatic one: a bar opening above would change it, and
        // the text would lose the focus as the bar closes.
        #[expect(
            clippy::disallowed_methods,
            reason = "named and addressed below, as the text"
        )]
        let mut response = ui.interact(visible, text_id.with("text"), Sense::click_and_drag());
        let id = response.id;
        if std::mem::take(&mut editor.wants_focus) {
            ui.memory_mut(|memory| memory.request_focus(id));
        }
        let focused = ui.memory(|memory| memory.has_focus(id));
        if focused {
            lock_focus(ui, id, bubbles::open(editor, self.completion.as_ref()));
        }
        input::pointer(ui, &response, editor, &frame);
        let keys = input::Keys {
            unit: self.unit,
            comment: self.comment,
            read_only: self.read_only,
            page: viewport.height(),
        };
        // While the pointer is on the list, a click on it moves the focus there: it stays shown.
        let listing = bubbles::open(editor, self.completion.as_ref())
            && (focused || bubbles::pointer_on_list(ui, id));
        if listing
            && focused
            && let Some(completion) = &self.completion
            && let Some(first) = bubbles::keys(ui, editor, completion)
        {
            *changed = Some(first);
        }
        if focused && let Some(first) = input::keyboard(ui, editor, &keys, &frame) {
            *changed = Some(changed.map_or(first, |earlier| earlier.min(first)));
        }
        if let Some(first) = menu(&response, editor, &keys, self.language) {
            *changed = Some(changed.map_or(first, |earlier| earlier.min(first)));
        }
        if let Some(first) = *changed {
            editor.lines.forget_from(first);
            editor.lines.measure(&editor.buffer, &frame);
            response.mark_changed();
        }
        let style = Style {
            paint: geometry::Paint {
                colours: self.colours,
                decorations: self.decorations,
            },
            marked: self.marked_lines,
            lit: self.lit_lines.clone(),
            hint: self.hint,
            gutter,
            focused,
        };
        let caret = paint(ui, editor, &frame, visible, &style);
        rows.extend(rows_on_screen(editor, &frame, visible));
        if focused {
            ask_input_method(ui, visible, caret);
        }
        if listing
            && changed.is_none()
            && let Some(completion) = &self.completion
            && let Some(first) = bubbles::list(ui, editor, completion, caret, id)
        {
            *changed = Some(first);
            editor.lines.forget_from(first);
            response.mark_changed();
        }
        if let Some(hover) = self.hover {
            bubbles::tooltip(ui, &response, editor, &frame, hover);
        }
        if std::mem::take(&mut editor.reveal) {
            ui.scroll_to_rect(caret.expand2(Vec2::new(char_width * 4.0, row_height)), None);
        }
        let label = self.label;
        let value = (editor.buffer.len() <= VALUE_UP_TO).then(|| editor.text());
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(egui::Role::MultilineTextInput, true, label);
            info.current_text_value.clone_from(&value);
            info
        });
        crate::widgets::address(ui, &response, self.author_id);
        response
    }
}

fn rows_on_screen<'a>(
    editor: &'a Editor,
    frame: &'a geometry::Frame,
    visible: Rect,
) -> impl Iterator<Item = Row> + 'a {
    editor.visible.clone().map(move |line| {
        let at = geometry::origin(&editor.lines, line, frame);
        Row {
            line,
            rect: Rect::from_min_size(
                Pos2::new(visible.left(), at.y),
                Vec2::new(visible.width(), frame.row_height),
            ),
        }
    })
}

/// The keys the text keeps: Tab and the arrows; Escape too while a list is open, which it
/// closes rather than leaving the text.
fn lock_focus(ui: &Ui, id: egui::Id, escape: bool) {
    let filter = egui::EventFilter {
        tab: true,
        horizontal_arrows: true,
        vertical_arrows: true,
        escape,
    };
    ui.memory_mut(|memory| memory.set_focus_lock_filter(id, filter));
}

/// Where the window puts the input method's window: at the caret.
fn ask_input_method(ui: &Ui, visible: Rect, caret: Rect) {
    let to_global = ui
        .ctx()
        .layer_transform_to_global(ui.layer_id())
        .unwrap_or_default();
    ui.ctx().output_mut(|output| {
        output.ime = Some(egui::output::IMEOutput {
            purpose: egui::IMEPurpose::Normal,
            rect: to_global * visible,
            cursor_rect: to_global * caret,
            should_interrupt_composition: false,
        });
    });
}

/// What a frame paints besides the text.
struct Style<'a> {
    paint: geometry::Paint<'a>,
    marked: &'a [usize],
    lit: Range<usize>,
    hint: &'a str,
    gutter: f32,
    focused: bool,
}

/// The visible lines: gutter, current line, selection, text, composition and caret. Answers
/// the caret's rectangle.
fn paint(
    ui: &Ui,
    editor: &mut Editor,
    frame: &geometry::Frame,
    visible: Rect,
    style: &Style<'_>,
) -> Rect {
    let p = palette(ui.ctx());
    let painter = ui.painter_at(visible);
    let selection = editor.buffer.selection();
    let selected = selection.range();
    let head_line = editor.buffer.line_of(selection.head);
    let first = editor.lines.line_at(visible.top() - frame.top);
    let last = editor.lines.line_at(visible.bottom() - frame.top);
    editor.visible = first..last + 1;
    let gutter_left = frame.text_left - PADDING - style.gutter;
    let selection_fill = ui.visuals().selection.bg_fill;
    for line in first..=last {
        let galley = geometry::galley(ui, &editor.buffer, line, frame, Some(&style.paint));
        let height = galley.rect.height().max(frame.row_height);
        editor.lines.set_height(line, height);
        let at = geometry::origin(&editor.lines, line, frame);
        if style.focused && selected.is_empty() && line == head_line {
            let row = Rect::from_min_size(
                Pos2::new(visible.left(), at.y),
                Vec2::new(visible.width(), height),
            );
            painter.rect_filled(row, 0.0, p.bg_2);
        }
        if style.lit.contains(&line) {
            let row = Rect::from_min_size(
                Pos2::new(visible.left(), at.y),
                Vec2::new(visible.width(), height),
            );
            painter.rect_filled(row, 0.0, p.blue.gamma_multiply(0.1));
            let bar = Rect::from_min_size(row.min, Vec2::new(3.0, height));
            painter.rect_filled(bar, 0.0, p.blue);
        }
        if style.gutter > 0.0 {
            gutter_cell(
                &painter,
                (line, line == head_line),
                at.y,
                gutter_left,
                style,
                frame,
            );
        }
        let start = editor.buffer.line_start(line);
        let length = editor.buffer.line(line).chars().count();
        if !selected.is_empty() && selected.start <= start + length && selected.end >= start {
            let from = selected.start.max(start) - start;
            let to = selected.end.min(start + length) - start;
            let past_end = selected.end > start + length;
            for rect in selection_rects(&galley, from..to, past_end, frame.char_width) {
                painter.rect_filled(rect.translate(at.to_vec2()), 2.0, selection_fill);
            }
        }
        painter.galley(at, galley, p.text_0);
    }
    if editor.buffer.is_empty() && !style.hint.is_empty() {
        let family = frame.font.family.clone();
        let hint = crate::widgets::text(ui, style.hint, frame.font.size, 400.0, family, p.text_2);
        painter.galley(geometry::origin(&editor.lines, 0, frame), hint, p.text_2);
    }
    let caret = geometry::caret_rect(ui, &editor.buffer, &editor.lines, selection.head, frame);
    let mut caret_x = caret.left();
    if !editor.preedit.is_empty() {
        let family = frame.font.family.clone();
        let composed = crate::widgets::text(
            ui,
            &editor.preedit,
            frame.font.size,
            400.0,
            family,
            p.text_0,
        );
        let width = composed.size().x;
        let under = Rect::from_min_size(caret.min, Vec2::new(width, caret.height()));
        painter.rect_filled(under, 0.0, p.bg_1);
        painter.galley(caret.min, composed, p.text_0);
        painter.hline(
            caret.left()..=caret.left() + width,
            caret.bottom() - 1.0,
            Stroke::new(1.0, p.text_1),
        );
        caret_x += width;
    }
    if style.focused {
        painter.vline(caret_x, caret.y_range(), Stroke::new(1.5, p.text_0));
    }
    caret
}

/// The line's number, brighter on the caret's line, and its mark.
fn gutter_cell(
    painter: &egui::Painter,
    (line, current): (usize, bool),
    top: f32,
    left: f32,
    style: &Style<'_>,
    frame: &geometry::Frame,
) {
    let p = palette(painter.ctx());
    let ink = if current { p.text_1 } else { p.text_2 };
    let number = painter.layout_no_wrap((line + 1).to_string(), frame.font.clone(), ink);
    let right = left + style.gutter - PADDING;
    painter.galley(Pos2::new(right - number.size().x, top), number, ink);
    if style.marked.binary_search(&line).is_ok() {
        let bar = Rect::from_min_size(
            Pos2::new(left + style.gutter - 3.0, top),
            Vec2::new(2.0, frame.row_height),
        );
        painter.rect_filled(bar, 1.0, p.amber_text);
    }
}

/// The selection's rectangles over a line's rows, local to the galley; past its end, the
/// line ending's width as well.
pub(crate) fn selection_rects(
    galley: &Galley,
    range: Range<usize>,
    past_end: bool,
    char_width: f32,
) -> Vec<Rect> {
    let mut rects = Vec::new();
    let mut row_start = 0;
    let rows = galley.rows.len();
    for (index, row) in galley.rows.iter().enumerate() {
        let row_end = row_start + row.char_count_including_newline().0;
        let from = range.start.max(row_start);
        let to = range.end.min(row_end);
        let last = index + 1 == rows;
        if from < to || (last && past_end) {
            let x0 = galley.pos_from_cursor(CCursor::new(from)).left();
            let mut x1 = galley.pos_from_cursor(CCursor::new(to)).left();
            if last && past_end {
                x1 += char_width;
            }
            rects.push(Rect::from_x_y_ranges(x0..=x1, row.min_y()..=row.max_y()));
        }
        row_start = row_end;
    }
    rects
}

/// Cut, copy, paste, select all. Paste asks the window for the clipboard: it arrives at the next
/// frame as an event the keys take.
fn menu(
    response: &Response,
    editor: &mut Editor,
    keys: &input::Keys<'_>,
    language: i18n::Language,
) -> Option<usize> {
    use i18n::keys::editor::menu as words;
    let mut edited = None;
    response.context_menu(|ui| {
        let tr = |key| i18n::translate(language, key, &[]);
        let writable = !keys.read_only;
        let entries = [
            (words::CUT, "editor-menu-cut", writable),
            (words::COPY, "editor-menu-copy", true),
            (words::PASTE, "editor-menu-paste", writable),
            (words::SELECT_ALL, "editor-menu-select-all", true),
        ];
        for (key, author_id, shown) in entries {
            if !shown {
                continue;
            }
            let label = tr(key);
            if !crate::widgets::menu_entry(ui, &label, author_id, false, false, false).clicked() {
                continue;
            }
            let selected = editor.buffer.selected_text();
            let ctx = ui.ctx().clone();
            match author_id {
                "editor-menu-cut" | "editor-menu-copy" if !selected.is_empty() => {
                    ctx.copy_text(selected);
                    if author_id == "editor-menu-cut" {
                        edited = editor
                            .buffer
                            .delete(text::Direction::Backward, text::Unit::Grapheme)
                            .map(|edited| edited.first_line);
                    }
                }
                "editor-menu-paste" => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
                "editor-menu-select-all" => editor.buffer.select_all(),
                _ => {}
            }
            ui.close();
            response.request_focus();
        }
    });
    edited
}

#[cfg(test)]
mod tests;
