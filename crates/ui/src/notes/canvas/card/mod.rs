//! A note's card, read like a small editor window: a title bar (the kind's icon, the title, and
//! what asks for attention), a preview of whole lines (3, 4 or 5, the library's choice: the card
//! is 120, 140 or 160 px), and a status line (where the note is, what it holds, when it changed).

mod actions;
mod bands;
mod body;

use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Galley, Pos2, Rect, Role, Sense, Stroke, StrokeKind,
    Ui, WidgetInfo, pos2, vec2,
};
use engine::folders::board::PreviewLines;
use engine::notes::model::DisplayNote;
use i18n::keys;

use crate::action::Actions;
use crate::services::time::Clock;
use crate::state::drawing_pictures::Thumbnails;
use crate::theme::{palette, spacing};
use crate::widgets::address;

/// What a card reads besides its note.
pub(super) struct Context<'a> {
    pub(super) language: i18n::Language,
    pub(super) calc: crate::notes::calc::Format,
    pub(super) clock: &'a dyn Clock,
    pub(super) thumbnails: &'a Thumbnails,
    /// The spaces and folders a note can be moved to or filed into.
    pub(super) library: &'a crate::state::spaces::SpacesState,
    pub(super) folders: &'a crate::state::folders::FoldersState,
    /// The card a first `Delete` armed: a second one sends it to the trash.
    pub(super) armed: Option<&'a str>,
    /// The ticked cards: while any is, every card shows its box.
    pub(super) selection: &'a crate::state::selection::SelectionState,
    pub(super) selecting: bool,
    /// How many lines a card shows: its height follows.
    pub(super) lines: PreviewLines,
    /// The note open in the editor beside the cards: its card stays lit.
    pub(super) opened: Option<&'a str>,
}

impl<'a> Context<'a> {
    /// `on_screen`: the cards the view draws, which decide whether every box shows.
    pub(super) fn of(state: &'a crate::state::AppState, on_screen: &[&str]) -> Self {
        Self {
            language: state.language,
            calc: state.calc_format(),
            clock: state.clock.as_ref(),
            thumbnails: &state.caches.thumbnails,
            library: &state.spaces,
            folders: &state.folders,
            armed: state.selection.armed(),
            selection: &state.selection,
            selecting: !state.selection.checked_in(on_screen).is_empty(),
            lines: state.preview_lines,
            opened: state.editor.open_id(),
        }
    }

    pub(super) fn card_height(&self) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "140, 160 or 180")]
        let height = self.lines.card_height() as f32;
        height
    }
}

/// Between the bands and the preview.
const GAP: f32 = 6.0;

pub(super) fn show(
    ui: &mut Ui,
    rect: Rect,
    note: &DisplayNote,
    context: &Context<'_>,
    (canvas_view, actions): (&mut super::CanvasView, &mut Actions),
) {
    let p = palette(ui.ctx());
    let title = if note.title.is_empty() {
        i18n::translate(context.language, keys::notes::UNTITLED, &[])
    } else {
        note.title.clone()
    };
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Article, true, &title));
    address(ui, &response, "note-card");
    // ⚠️ The click surface is a layer under the card's controls, not the card: what is drawn
    // after it, a task's box, takes the click first.
    // ⚠️ Named after the note, not its place: the focus follows a card the list moves. Its layer
    // too: the silhouette of a drag is the same card, drawn above.
    #[expect(
        clippy::disallowed_methods,
        reason = "named and addressed just below, as the card"
    )]
    let open = ui.interact(
        rect,
        egui::Id::unique(("note-card-open", &note.id)).with(ui.layer_id()),
        Sense::click(),
    );
    open.widget_info(|| WidgetInfo::labeled(Role::Button, true, &title));
    address(ui, &open, "note-card-open");
    crate::theme::focus::ring(ui, &open, CornerRadius::same(10));
    if canvas_view.request.as_deref() == Some(note.id.as_str()) {
        open.request_focus();
        canvas_view.request = None;
    }
    // Not `has_focus`, which also asks the window's focus: under xvfb no card would ever have it.
    if crate::widgets::takes_keys(ui, &open) {
        canvas_view.focused = Some(note.id.clone());
        // The arrows and Escape are the canvas's table's: egui would move the focus itself.
        let kept = egui::EventFilter {
            horizontal_arrows: true,
            vertical_arrows: true,
            escape: true,
            ..egui::EventFilter::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(open.id, kept));
    }
    if open.clicked() {
        clicked(ui, note, canvas_view, actions);
    }
    let checked = context.selection.is_checked(&note.id);
    let armed = context.armed == Some(note.id.as_str());
    let hovered = ui.rect_contains_pointer(rect);
    let opened = context.opened == Some(note.id.as_str());
    let (fill, edge) = if armed {
        (p.bg_2, p.red)
    } else if opened || checked {
        (p.bg_2, p.amber_edge)
    } else if hovered {
        (p.bg_2, p.line)
    } else {
        (p.bg_1, p.line_soft)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        fill,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    if opened {
        let tint = p.amber_fill.gamma_multiply(0.08);
        ui.painter().rect(
            rect,
            CornerRadius::same(10),
            tint,
            Stroke::new(1.5, edge),
            StrokeKind::Inside,
        );
    }

    let inner = rect.shrink(1.0);
    let top = Rect::from_min_size(inner.min, vec2(inner.width(), bands::TOP_HEIGHT));
    let bottom = Rect::from_min_max(
        pos2(inner.left(), inner.bottom() - bands::BOTTOM_HEIGHT),
        inner.max,
    );
    bands::frame(ui, (top, bottom), hovered, armed);
    // Not under the pointer alone: the box would come and go as the pointer crosses the card.
    let ticking = checked || context.selecting;
    if bands::top(ui, top, (note, &title), context, (checked, ticking)) {
        actions.push(crate::action::Action::Notes(crate::notes::Action::Check(
            note.id.clone(),
        )));
    }
    let inset = spacing(ui.ctx()).card.min(12.0);
    let room = Rect::from_min_max(
        pos2(inner.left() + inset, top.bottom() + GAP),
        pos2(inner.right() - inset, bottom.top() - GAP),
    );
    body::show(ui, room, note, (context, context.lines.count()), actions);
    if armed {
        bands::arming(ui, bottom, context.language);
    } else {
        bands::bottom(ui, bottom, note, context);
    }
    actions::show(ui, rect, note, (context, ticking), actions);
}

/// Opened by a click; ticked with Ctrl, or as far as the anchor with Shift.
fn clicked(ui: &Ui, note: &DisplayNote, canvas_view: &super::CanvasView, actions: &mut Actions) {
    let modifiers = ui.input(|input| input.modifiers);
    let id = note.id.clone();
    let asked = if modifiers.command {
        crate::notes::Action::Check(id)
    } else if modifiers.shift {
        crate::notes::Action::CheckRange {
            id,
            anchor: canvas_view.anchor.clone(),
        }
    } else {
        crate::notes::Action::OpenNote(id)
    };
    actions.push(crate::action::Action::Notes(asked));
}

/// A text's face: its size, its CSS weight, its family, and its line height as a factor.
pub(super) struct Font {
    size: f32,
    weight: f32,
    family: FontFamily,
    line: f32,
}

impl Font {
    pub(super) fn proportional(size: f32, weight: f32, line: f32) -> Self {
        Self {
            size,
            weight,
            family: FontFamily::Proportional,
            line,
        }
    }

    pub(super) fn mono(size: f32, line: f32) -> Self {
        Self {
            size,
            weight: 400.0,
            family: FontFamily::Monospace,
            line,
        }
    }

    pub(super) fn height(&self) -> f32 {
        self.size * self.line
    }

    pub(super) fn format(&self, colour: Color32) -> TextFormat {
        TextFormat {
            font_id: FontId::new(self.size, self.family.clone()),
            color: colour,
            line_height: Some(self.height()),
            coords: egui::epaint::text::VariationCoords::new([(*b"wght", self.weight)]),
            ..TextFormat::default()
        }
    }
}

/// `words` in `width`, `rows` lines at most, an ellipsis where it is cut.
pub(super) fn galley(
    ui: &Ui,
    words: &str,
    font: &Font,
    colour: Color32,
    width: f32,
    rows: usize,
) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(words.to_string(), font.format(colour));
    job.wrap = TextWrapping {
        max_width: width.max(1.0),
        max_rows: rows,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// A text that only reads: its node, found by `author_id`.
pub(super) fn label(ui: &mut Ui, rect: Rect, words: &str, author_id: &str) {
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    address(ui, &response, author_id);
}

/// `galley` drawn at `at`, and its node.
pub(super) fn text_at(
    ui: &mut Ui,
    at: Pos2,
    galley: Arc<Galley>,
    words: &str,
    author_id: &str,
) -> Rect {
    let rect = Rect::from_min_size(at, galley.size());
    label(ui, rect, words, author_id);
    ui.painter().galley(at, galley, Color32::TRANSPARENT);
    rect
}

#[cfg(test)]
mod tests;
