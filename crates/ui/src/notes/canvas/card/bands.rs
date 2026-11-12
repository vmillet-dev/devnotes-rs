//! A card's two bands, as a small window's: the top one is its title bar — the kind's icon, a
//! snippet's format and the title, then on the right the priority, the pin and, while cards are
//! ticked, the box. The bottom one is its status line — where the note is on the left; on the
//! right its fields to fill, its attachments, and when it changed. Neither is ever empty.

use egui::{
    Color32, CornerRadius, Rect, Role, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::model::{DisplayNote, NoteFooter};
use i18n::{Message, keys};

use super::{Context, Font, galley, label, text_at};
use crate::notes::ui::{folder_hue, format_label, kind_icon, priority_bars, tick_box};
use crate::services::time;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{MIN_TARGET, address};

/// The title bar: a little taller than the status line, for a title at 13 px.
pub(super) const TOP_HEIGHT: f32 = 28.0;
pub(super) const BOTTOM_HEIGHT: f32 = 22.0;
/// From the band's edges to what it holds; the kind's tile sits closer, as a window's icon.
const INSET: f32 = 10.0;
const TILE_INSET: f32 = 6.0;
const GAP: f32 = 6.0;

/// No fill but under the pointer; a hairline between each band and the rest of the card.
pub(super) fn frame(ui: &Ui, (top, bottom): (Rect, Rect), hovered: bool, armed: bool) {
    let p = palette(ui.ctx());
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(
            top,
            CornerRadius {
                nw: 9,
                ne: 9,
                sw: 0,
                se: 0,
            },
            p.bg_3,
        );
    }
    let corner = CornerRadius {
        nw: 0,
        ne: 0,
        sw: 9,
        se: 9,
    };
    if armed {
        painter.rect_filled(bottom, corner, p.red.gamma_multiply(0.12));
    } else if hovered {
        painter.rect_filled(bottom, corner, p.bg_3);
    }
    let rule = Stroke::new(1.0, p.line_soft);
    painter.hline(top.x_range(), top.bottom() - 0.5, rule);
    painter.hline(bottom.x_range(), bottom.top() + 0.5, rule);
}

/// The top band. `true` when its box was clicked.
pub(super) fn top(
    ui: &mut Ui,
    band: Rect,
    (note, title): (&DisplayNote, &str),
    context: &Context<'_>,
    (checked, ticking): (bool, bool),
) -> bool {
    let language = context.language;
    let ticked = tick(ui, band, title, (checked, ticking), language);
    // The box takes the right end; the pin moves over for it.
    let right_end = if ticking {
        band.right() - MIN_TARGET - 2.0
    } else {
        band.right() - INSET
    };
    let mut right = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_max(band.min, pos2(right_end, band.bottom())))
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 7.0;
    if note.pinned {
        pin(&mut right, language);
    }
    priority_bars(&mut right, note.priority, language);
    if note.kind == NoteKind::Checklist {
        progress(&mut right, &note.items, language);
    }
    let title_end = right.min_rect().left() - GAP;

    let icon = Rect::from_min_size(
        pos2(band.left() + TILE_INSET, band.center().y - 9.0),
        vec2(18.0, 18.0),
    );
    let room = Rect::from_min_max(icon.min, pos2(title_end.max(icon.right()), icon.bottom()));
    let mut left = ui.new_child(
        UiBuilder::new()
            .max_rect(room)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left.spacing_mut().item_spacing.x = GAP;
    kind_icon(&mut left, note.kind, language);
    format_label(&mut left, note.kind, note.language, language);
    // An untitled note has no title, never « Untitled »: its icon stands alone.
    if !note.title.is_empty() {
        let p = palette(ui.ctx());
        let font = Font::proportional(13.0, 600.0, 18.0 / 13.0);
        let from = left.min_rect().right() + 8.0;
        let shown = galley(ui, &note.title, &font, p.text_0, title_end - from, 1);
        let at = pos2(from, band.center().y - shown.size().y / 2.0);
        text_at(ui, at, shown, &note.title, "note-card-title");
    }
    ticked
}

/// A list's « 1/4 », after its title.
pub(super) fn progress(ui: &mut Ui, items: &[ChecklistItem], language: i18n::Language) {
    let p = palette(ui.ctx());
    let done = items.iter().filter(|item| item.done).count();
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX).into();
    let words = i18n::translate(
        language,
        keys::notes::CHECKLIST_PROGRESS,
        &[("done", count(done)), ("total", count(items.len()))],
    );
    let font = Font {
        weight: 600.0,
        ..Font::mono(10.0, 1.2)
    };
    let shown = galley(ui, &words, &font, p.text_2, f32::INFINITY, 1);
    let (rect, _) = ui.allocate_exact_size(shown.size(), Sense::hover());
    text_at(ui, rect.min, shown, &words, "note-card-progress");
}

/// An icon and its count, named by what they count.
fn counted(ui: &mut Ui, (icon, ink): (Icon, Color32), count: u32, words: &str, author_id: &str) {
    let figure = galley(
        ui,
        &count.to_string(),
        &Font::mono(10.0, 1.2),
        ink,
        f32::INFINITY,
        1,
    );
    let size = vec2(12.0 + 3.0 + figure.size().x, 12.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    label(ui, rect, words, author_id);
    let glyph = Rect::from_min_size(rect.min, vec2(12.0, 12.0));
    icons::paint(ui, glyph, icon, ink);
    let at = pos2(glyph.right() + 3.0, rect.center().y - figure.size().y / 2.0);
    ui.painter().galley(at, figure, Color32::TRANSPARENT);
}

fn pin(ui: &mut Ui, language: i18n::Language) {
    let words = i18n::translate(language, keys::notes::PINNED_STATE, &[]);
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 12.0), Sense::hover());
    label(ui, rect, &words, "note-card-pin");
    icons::paint(ui, rect, Icon::PinFilled, palette(ui.ctx()).amber_text);
}

/// The card's box, last in the top band while cards are ticked: nothing on its left moves.
/// ⚠️ Allocated on every frame, shown or not: a widget that comes and goes moves the ids of
/// every one after it, and the ⋯'s menu lost its own.
fn tick(
    ui: &mut Ui,
    band: Rect,
    title: &str,
    (checked, shown): (bool, bool),
    language: i18n::Language,
) -> bool {
    let p = palette(ui.ctx());
    let label = i18n::translate(
        language,
        keys::notes::SELECT_NOTE,
        &[("title", title.into())],
    );
    let target = Rect::from_center_size(
        pos2(band.right() - MIN_TARGET / 2.0 - 1.0, band.center().y),
        egui::Vec2::splat(MIN_TARGET),
    );
    let sense = if shown {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.allocate_rect(target, sense);
    if !shown {
        return false;
    }
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, true, checked, &label));
    address(ui, &response, "note-card-check");
    crate::theme::focus::ring(ui, &response, CornerRadius::same(4));
    let colour = if checked { p.amber_text } else { p.text_2 };
    let square = Rect::from_center_size(target.center(), egui::Vec2::splat(13.0));
    tick_box(ui, square, checked, colour);
    response.clicked()
}

/// The bottom band: where, on the left; what the note holds and when it changed, on the right.
pub(super) fn bottom(ui: &mut Ui, band: Rect, note: &DisplayNote, context: &Context<'_>) {
    let p = palette(ui.ctx());
    let language = context.language;
    let inner = band.shrink2(vec2(INSET, 0.0));
    let mut right = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 8.0;
    date(&mut right, note, context);
    if note.attachment_count > 0 {
        let count = note.attachment_count;
        let words = i18n::translate(
            language,
            keys::notes::ATTACHMENTS_COUNT,
            &[("count", count.into())],
        );
        let ink = p.text_2;
        counted(
            &mut right,
            (Icon::Paperclip, ink),
            count,
            &words,
            "note-card-clip",
        );
    }
    if !note.placeholders.is_empty() {
        let count = u32::try_from(note.placeholders.len()).unwrap_or(u32::MAX);
        let words = i18n::translate(
            language,
            keys::notes::PLACEHOLDERS_COUNT,
            &[("count", count.into())],
        );
        let ink = p.amber_text;
        counted(
            &mut right,
            (Icon::Zap, ink),
            count,
            &words,
            "note-card-fields",
        );
    }
    let room = (right.min_rect().left() - GAP - inner.left()).max(0.0);
    if let Some(place) = place_of(ui, note, context) {
        draw_place(ui, &place, pos2(inner.left(), band.center().y), room);
    }
}

/// « 4 h », in red when the note expires soon.
fn date(ui: &mut Ui, note: &DisplayNote, context: &Context<'_>) {
    let p = palette(ui.ctx());
    let (short, long) = when(ui, note, context);
    let ink = if note.expiring_soon { p.red } else { p.text_2 };
    let weight = if note.expiring_soon { 600.0 } else { 400.0 };
    let font = Font {
        weight,
        ..Font::mono(10.0, 1.2)
    };
    let shown = galley(ui, &short, &font, ink, f32::INFINITY, 1);
    let (rect, _) = ui.allocate_exact_size(shown.size(), Sense::hover());
    text_at(ui, rect.min, shown, &long, "note-card-when");
}

/// In the bottom band's place, real words and both keys: what finishes the deletion, and the
/// way out.
pub(super) fn arming(ui: &mut Ui, band: Rect, language: i18n::Language) {
    let p = palette(ui.ctx());
    let words = i18n::translate(language, keys::notes::DELETE_ARMED, &[]);
    let inner = band.shrink2(vec2(INSET, 0.0));
    let shown = galley(
        ui,
        &words,
        &Font::proportional(10.0, 600.0, 1.2),
        p.red,
        inner.width(),
        1,
    );
    let at = pos2(inner.left(), band.center().y - shown.size().y / 2.0);
    text_at(ui, at, shown, &words, "note-card-arming");
}

/// « 4 h » on the card, « modifiée il y a 4 h » for the screen reader and the scenarios — and
/// asks to be drawn again when it reads otherwise.
fn when(ui: &Ui, note: &DisplayNote, context: &Context<'_>) -> (String, String) {
    let tr = |message: &Message| message.text(context.language);
    let now = context.clock.now();
    match &note.footer {
        NoteFooter::Expiry { at } => {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_mins(1));
            let short = i18n::time::short_expiry(*at, now, &chrono::Local);
            let long = i18n::time::expiry(*at, now, &chrono::Local);
            (tr(&short), tr(&long))
        }
        // The source is the editor's to show: a card's corner says when.
        NoteFooter::Source { .. } => {
            aged(ui, note.updated_at, keys::notes::footer::MODIFIED, context)
        }
        NoteFooter::Age { at } => aged(ui, *at, keys::notes::footer::MODIFIED, context),
        NoteFooter::Created { at } => aged(ui, *at, keys::notes::footer::CREATED, context),
    }
}

fn aged(
    ui: &Ui,
    at: chrono::DateTime<chrono::Utc>,
    prefix: i18n::Key,
    context: &Context<'_>,
) -> (String, String) {
    let (label, wait) = time::relative(at, context.clock);
    ui.ctx().request_repaint_after(wait);
    let short = i18n::time::short(at, context.clock.now()).text(context.language);
    let long = format!(
        "{} {}",
        i18n::translate(context.language, prefix, &[]),
        label.text(context.language)
    );
    (short, long)
}

/// Where the note sits, as the band writes it.
struct Place {
    words: String,
    /// The folder's colour square, when it is in one.
    hue: Option<Color32>,
    /// Said on hover and to the screen reader: the whole path.
    path: String,
    author_id: &'static str,
}

/// The folder, after its space under « All spaces »; with no folder, the space; inside the
/// folder's own view (or its zone), nothing — the breadcrumb says it.
fn place_of(ui: &Ui, note: &DisplayNote, context: &Context<'_>) -> Option<Place> {
    let every_space = context.library.active_space().is_none();
    let space = context
        .library
        .spaces()
        .iter()
        .find(|space| space.id == note.space_id)
        .map(|space| space.name.clone());
    if let Some(folder) = &note.folder {
        let mut path = folder.path.clone();
        let mut words = folder.name.clone();
        if every_space && let Some(space) = &space {
            words = format!("{space} › {words}");
            path.insert(0, space.clone());
        }
        return Some(Place {
            words,
            hue: Some(folder_hue(ui, folder.colour)),
            path: path.join(" › "),
            author_id: "note-card-folder",
        });
    }
    if note.folder_id.is_some() {
        return None;
    }
    space.map(|name| Place {
        words: name.clone(),
        hue: None,
        path: name,
        author_id: "note-card-space",
    })
}

fn place_font() -> Font {
    Font::proportional(10.0, 400.0, 1.2)
}

/// Answers its right edge.
fn draw_place(ui: &mut Ui, place: &Place, left_middle: egui::Pos2, room: f32) -> f32 {
    let p = palette(ui.ctx());
    let mut left = left_middle.x;
    let middle = left_middle.y;
    if let Some(hue) = place.hue {
        let swatch = Rect::from_center_size(pos2(left + 3.5, middle), vec2(7.0, 7.0));
        ui.painter().rect_filled(swatch, CornerRadius::same(2), hue);
        left = swatch.right() + 5.0;
    }
    let width = (room - (left - left_middle.x)).max(1.0);
    let text = galley(ui, &place.words, &place_font(), p.text_2, width, 1);
    let at = pos2(left, middle - text.size().y / 2.0);
    let rect = Rect::from_min_max(pos2(left_middle.x, at.y), at + text.size());
    let response = ui.allocate_rect(rect, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, &place.path));
    address(ui, &response, place.author_id);
    ui.painter().galley(at, text, Color32::TRANSPARENT);
    crate::widgets::tooltip(response, &place.path);
    rect.right()
}
