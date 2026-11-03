//! The card's last line: when, or where from; the marks, which never give way; the folder's
//! chip and the tags, which do, a tag going whole or not at all, and how many did not fit.

use egui::{Color32, CornerRadius, Rect, Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2};
use engine::folders::model::FolderColour;
use engine::notes::model::{DisplayNote, NoteFooter};
use engine::notes::view::SearchField;
use i18n::{Message, keys};

use super::{Context, Font, galley, label, text_at};
use crate::notes::ui::priority_pill;
use crate::services::time;
use crate::theme::icons::{self, Icon};
use crate::theme::palette;

const MAX_VISIBLE_TAGS: usize = 2;
const GAP: f32 = 6.0;

pub(super) fn show(ui: &mut Ui, rect: Rect, note: &DisplayNote, context: &Context<'_>) {
    let p = palette(ui.ctx());
    let middle = rect.center().y;
    let font = Font::proportional(10.5, 400.0, 1.2);
    let when = when(ui, note, context);
    let ink = if note.expiring_soon { p.red } else { p.text_2 };
    let when = galley(ui, &when, &font, ink, rect.width() * 0.7, 1);
    let words = when.text().to_string();
    let mut left = text_at(
        ui,
        pos2(rect.left(), middle - when.size().y / 2.0),
        when,
        &words,
        "note-card-when",
    )
    .right()
        + GAP;
    left = marks(ui, rect, left, note, context.language);
    let tags_room = tags_width(ui, note);
    if let Some(folder) = &note.folder {
        let room = (rect.right() - left - tags_room).max(24.0);
        left = folder_chip(
            ui,
            pos2(left, middle),
            room,
            &folder.name,
            folder.colour,
            &folder.path.join(" › "),
        ) + GAP;
    }
    tags(
        ui,
        Rect::from_min_max(pos2(left, rect.top()), rect.max),
        note,
        context.language,
    );
}

/// Its source, its deadline, or how long ago it changed — and asks to be drawn again when that
/// reads otherwise.
fn when(ui: &Ui, note: &DisplayNote, context: &Context<'_>) -> String {
    let tr = |message: &Message| message.text(context.language);
    match &note.footer {
        NoteFooter::Source { value } => value.clone(),
        NoteFooter::Expiry { at } => {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_mins(1));
            tr(&i18n::time::expiry(
                *at,
                context.clock.now(),
                &chrono::Local,
            ))
        }
        NoteFooter::Age { at } | NoteFooter::Created { at } => {
            let (label, wait) = time::relative(*at, context.clock);
            ui.ctx().request_repaint_after(wait);
            let prefix = match note.footer {
                NoteFooter::Created { .. } => keys::notes::footer::CREATED,
                _ => keys::notes::footer::MODIFIED,
            };
            format!(
                "{} {}",
                i18n::translate(context.language, prefix, &[]),
                tr(&label)
            )
        }
    }
}

/// Priority, pin, fields, attachments: answers where they end.
fn marks(ui: &mut Ui, rect: Rect, left: f32, note: &DisplayNote, language: i18n::Language) -> f32 {
    let p = palette(ui.ctx());
    let mut left = left;
    let middle = rect.center().y;
    let row = Rect::from_min_max(pos2(left, rect.top()), rect.max);
    let mut marks_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(row)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    marks_ui.spacing_mut().item_spacing.x = 5.0;
    if let Some(pill) = priority_pill(&mut marks_ui, note.priority, language) {
        left = pill.rect.right() + 5.0;
    }
    let tr = |key, count: u32| i18n::translate(language, key, &[("count", count.into())]);
    let icon =
        |ui: &mut Ui, icon: Icon, colour: Color32, words: &str, author_id: &str, left: &mut f32| {
            let at = Rect::from_min_size(pos2(*left, middle - 6.0), vec2(12.0, 12.0));
            icons::paint(ui, at, icon, colour);
            label(ui, at, words, author_id);
            *left = at.right() + 5.0;
        };
    if note.pinned {
        let words = i18n::translate(language, keys::notes::PINNED_STATE, &[]);
        icon(ui, Icon::Pin, p.text_2, &words, "note-card-pin", &mut left);
    }
    if !note.placeholders.is_empty() {
        let count = u32::try_from(note.placeholders.len()).unwrap_or(u32::MAX);
        let words = tr(keys::notes::PLACEHOLDERS_COUNT, count);
        icon(
            ui,
            Icon::Zap,
            p.amber_text,
            &words,
            "note-card-fields",
            &mut left,
        );
    }
    if note.attachment_count > 0 {
        let words = tr(keys::notes::ATTACHMENTS_COUNT, note.attachment_count);
        icon(
            ui,
            Icon::Paperclip,
            p.text_2,
            &words,
            "note-card-clip",
            &mut left,
        );
        let count = galley(
            ui,
            &note.attachment_count.to_string(),
            &Font::proportional(10.5, 400.0, 1.2),
            p.text_2,
            f32::INFINITY,
            1,
        );
        let width = count.size().x;
        ui.painter().galley(
            pos2(left - 3.0, middle - count.size().y / 2.0),
            count,
            Color32::TRANSPARENT,
        );
        left += width + 2.0;
    }
    if left > rect.left() {
        left + GAP - 5.0
    } else {
        left
    }
}

/// A neutral pill with a square swatch: a tinted one is what a tag is. Answers its right edge.
fn folder_chip(
    ui: &mut Ui,
    middle: egui::Pos2,
    room: f32,
    name: &str,
    colour: FolderColour,
    path: &str,
) -> f32 {
    let p = palette(ui.ctx());
    let hue = match colour {
        FolderColour::Blue => p.blue,
        FolderColour::Amber => p.amber_text,
        FolderColour::Purple => p.purple,
        FolderColour::Green => p.green,
        FolderColour::Red => p.red,
    };
    let text = galley(
        ui,
        name,
        &Font::proportional(9.5, 400.0, 1.2),
        p.text_1,
        (room - 7.0 - 7.0 - 5.0 - 7.0).max(1.0),
        1,
    );
    let size = vec2(7.0 + 7.0 + 5.0 + text.size().x + 7.0, text.size().y + 2.0);
    let rect = Rect::from_min_size(pos2(middle.x, middle.y - size.y / 2.0), size);
    label(ui, rect, path, "note-card-folder");
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        p.bg_2,
        Stroke::new(1.0, p.line),
        StrokeKind::Inside,
    );
    let swatch = Rect::from_center_size(pos2(rect.left() + 7.0 + 3.5, middle.y), vec2(7.0, 7.0));
    ui.painter().rect_filled(swatch, CornerRadius::same(2), hue);
    ui.painter().galley(
        pos2(swatch.right() + 5.0, middle.y - text.size().y / 2.0),
        text,
        Color32::TRANSPARENT,
    );
    rect.right()
}

/// Two tags, the one a search found among them, each whole; `+N` for the others.
fn tags(ui: &mut Ui, rect: Rect, note: &DisplayNote, language: i18n::Language) {
    let p = palette(ui.ctx());
    let font = Font::mono(9.5, 14.0 / 9.5);
    let from = window_start(note);
    let candidates = &note.tags[from..note.tags.len().min(from + MAX_VISIBLE_TAGS)];
    let shown: Vec<_> = candidates
        .iter()
        .map(|tag| galley(ui, &format!("#{tag}"), &font, p.text_2, f32::INFINITY, 1))
        .collect();
    let more_of =
        |count: usize| galley(ui, &format!("+{count}"), &font, p.text_2, f32::INFINITY, 1);
    // Whole tags, from the first, while they fit beside the `+N` of the rest.
    let mut fitting = shown.len();
    while fitting > 0 {
        let hidden = note.tags.len() - fitting;
        let more = if hidden > 0 {
            more_of(hidden).size().x + 4.0
        } else {
            0.0
        };
        let width: f32 = shown[..fitting].iter().map(|tag| tag.size().x).sum::<f32>()
            + 4.0 * f32::from(u16::try_from(fitting - 1).unwrap_or(u16::MAX));
        if width + more <= rect.width() {
            break;
        }
        fitting -= 1;
    }
    let hidden = note.tags.len() - fitting;
    let middle = rect.center().y;
    let mut right = rect.right();
    if hidden > 0 {
        let more = more_of(hidden);
        right -= more.size().x;
        let words = i18n::translate(
            language,
            keys::notes::MORE_TAGS,
            &[("count", u32::try_from(hidden).unwrap_or(u32::MAX).into())],
        );
        text_at(
            ui,
            pos2(right, middle - more.size().y / 2.0),
            more,
            &words,
            "note-card-tags-more",
        );
        right -= 4.0;
    }
    let width: f32 = shown[..fitting]
        .iter()
        .map(|tag| tag.size().x + 4.0)
        .sum::<f32>()
        - 4.0;
    let mut left = right - width.max(0.0);
    let words: Vec<String> = candidates[..fitting]
        .iter()
        .map(|tag| format!("#{tag}"))
        .collect();
    label(
        ui,
        Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom())),
        &words.join(" "),
        "note-card-tags",
    );
    for tag in shown.into_iter().take(fitting) {
        let width = tag.size().x;
        ui.painter().galley(
            pos2(left, middle - tag.size().y / 2.0),
            tag,
            Color32::TRANSPARENT,
        );
        left += width + 4.0;
    }
}

/// What the tags need at most: the folder's chip takes only what they leave.
fn tags_width(ui: &Ui, note: &DisplayNote) -> f32 {
    let p = palette(ui.ctx());
    let font = Font::mono(9.5, 14.0 / 9.5);
    let from = window_start(note);
    note.tags[from..note.tags.len().min(from + MAX_VISIBLE_TAGS)]
        .iter()
        .map(|tag| {
            galley(ui, &format!("#{tag}"), &font, p.text_2, f32::INFINITY, 1)
                .size()
                .x
                + 4.0
        })
        .sum()
}

/// Where the two tags start for the one a search found to be among them.
fn window_start(note: &DisplayNote) -> usize {
    let Some(hit) = note
        .search_hit
        .as_ref()
        .filter(|hit| hit.field == SearchField::Tag)
    else {
        return 0;
    };
    let needle = hit.excerpt.strip_suffix('…').unwrap_or(&hit.excerpt);
    let Some(at) = note.tags.iter().position(|tag| tag.starts_with(needle)) else {
        return 0;
    };
    if at < MAX_VISIBLE_TAGS {
        return 0;
    }
    at.min(note.tags.len().saturating_sub(MAX_VISIBLE_TAGS))
}
