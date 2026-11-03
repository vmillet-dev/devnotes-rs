//! What a card shows of its note, by its kind: a snippet's first lines, a Note's outline, a
//! list's progress and two of its items, a sheet's last results, a drawing's picture — or the
//! line a search found, when that is not code.

use egui::{
    Color32, CornerRadius, Rect, Role, Sense, Stroke, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};
use engine::notes::checklist::ChecklistItem;
use engine::notes::kind::NoteKind;
use engine::notes::model::{DisplayNote, OutlineLine};
use engine::notes::view::SearchField;
use i18n::keys;

use super::{Context, Font, galley, label, text_at};
use crate::action::{Action, Actions};
use crate::notes::calc;
use crate::notes::ui::tick_box;
use crate::state::drawing_pictures::{Stamp, Thumbnail};
use crate::theme::palette;
use crate::widgets::{address, interact};

const MAX_VISIBLE_ITEMS: usize = 2;
/// A task's marks, as the screen reader is told them; drawn as boxes.
const TICKED: &str = "☑";
const UNTICKED: &str = "☐";

pub(super) fn show(
    ui: &mut Ui,
    rect: Rect,
    note: &DisplayNote,
    context: &Context<'_>,
    actions: &mut Actions,
) {
    let hit = note.search_hit.as_ref();
    match note.kind {
        NoteKind::Checklist => checklist(ui, rect, note, context.language, actions),
        NoteKind::Drawing if hit.is_none() => drawing(ui, rect, note, context, actions),
        NoteKind::Calc if hit.is_none() && !note.calc.is_empty() => {
            calc_lines(ui, rect, note, context.language);
        }
        NoteKind::Note if hit.is_none() => outline(ui, rect, &note.outline),
        NoteKind::Snippet if hit.is_none_or(|hit| hit.field == SearchField::Body) => {
            let shown = hit.map_or(note.content.as_str(), |hit| hit.excerpt.as_str());
            code(ui, rect, shown);
        }
        _ => {
            let tag = hit.is_some_and(|hit| hit.field == SearchField::Tag);
            let words = hit.map_or(note.content.as_str(), |hit| hit.excerpt.as_str());
            prose(ui, rect, words, tag, "note-card-hit");
        }
    }
}

/// Monospaced, a line a row, cut at the card's edge rather than wrapped.
fn code(ui: &mut Ui, rect: Rect, content: &str) {
    let p = palette(ui.ctx());
    let font = Font::mono(11.0, 1.55);
    label(ui, rect, content, "note-card-snippet");
    let painter = ui.painter().with_clip_rect(rect);
    let mut top = rect.top();
    for line in content.lines() {
        if top + font.height() > rect.bottom() + 0.5 {
            break;
        }
        let shown = galley(ui, line, &font, p.text_2, f32::INFINITY, 1);
        painter.galley(pos2(rect.left(), top), shown, Color32::TRANSPARENT);
        top += font.height();
    }
}

/// The line a tag, an item or a Note's text put on the results: prose, never painted as code.
fn prose(ui: &mut Ui, rect: Rect, words: &str, tag: bool, author_id: &str) {
    let p = palette(ui.ctx());
    let font = Font::proportional(11.0, 400.0, 1.55);
    let rows = rows_in(rect.height(), font.height());
    let mut job = egui::text::LayoutJob::default();
    if tag {
        job.append("# ", 0.0, font.format(p.text_2));
    }
    job.append(words, 0.0, font.format(p.text_1));
    job.wrap = egui::text::TextWrapping {
        max_width: rect.width(),
        max_rows: rows,
        break_anywhere: false,
        overflow_character: Some('…'),
    };
    let shown = ui.fonts_mut(|fonts| fonts.layout_job(job));
    text_at(ui, rect.min, shown, words, author_id);
}

fn rows_in(height: f32, line: f32) -> usize {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "floored"
    )]
    let rows = (height / line).floor().max(1.0) as usize;
    rows
}

/// A Note's lines as the core read them: headings, text, items and tasks with their markers,
/// code. A line that does not fit is not drawn — never half a line.
fn outline(ui: &mut Ui, rect: Rect, lines: &[OutlineLine]) {
    let p = palette(ui.ctx());
    let words: Vec<String> = lines.iter().map(plain).collect();
    label(ui, rect, &words.join("\n"), "note-card-outline");
    let mut top = rect.top();
    for line in lines {
        let (font, colour, marker, depth, text) = match line {
            OutlineLine::Heading { text } => (
                Font::proportional(12.0, 600.0, 1.45),
                p.text_0,
                None,
                0,
                text,
            ),
            OutlineLine::Text { text } => (
                Font::proportional(12.0, 400.0, 1.45),
                p.text_1,
                None,
                0,
                text,
            ),
            OutlineLine::Item {
                text,
                depth,
                marker,
            } => (
                Font::proportional(12.0, 400.0, 1.45),
                p.text_1,
                Some(marker.clone()),
                *depth,
                text,
            ),
            OutlineLine::Task { text, depth, done } => {
                let marker = if *done { TICKED } else { UNTICKED };
                (
                    Font::proportional(12.0, 400.0, 1.45),
                    p.text_1,
                    Some(marker.to_string()),
                    *depth,
                    text,
                )
            }
            OutlineLine::Code { text } => (Font::mono(11.0, 1.45), p.text_2, None, 0, text),
        };
        if top + font.height() > rect.bottom() + 0.5 {
            break;
        }
        #[expect(clippy::cast_precision_loss, reason = "a nesting depth")]
        let mut left = rect.left() + depth as f32 * 12.0;
        if let Some(marker) = marker
            .as_deref()
            .filter(|marker| [TICKED, UNTICKED].contains(marker))
        {
            let at = Rect::from_min_size(
                pos2(left, top + (font.height() - 11.0) / 2.0),
                vec2(11.0, 11.0),
            );
            tick_box(ui, at, marker == TICKED, p.purple);
            left += 11.0 + 6.0;
        } else if let Some(marker) = marker {
            let shown = galley(ui, &marker, &font, p.purple, f32::INFINITY, 1);
            let width = shown.size().x;
            ui.painter()
                .galley(pos2(left, top), shown, Color32::TRANSPARENT);
            left += width + 6.0;
        }
        let shown = galley(ui, text, &font, colour, rect.right() - left, 1);
        ui.painter()
            .with_clip_rect(rect)
            .galley(pos2(left, top), shown, Color32::TRANSPARENT);
        top += font.height() + 3.0;
    }
}

fn plain(line: &OutlineLine) -> String {
    match line {
        OutlineLine::Heading { text } | OutlineLine::Text { text } | OutlineLine::Code { text } => {
            text.clone()
        }
        OutlineLine::Item { text, marker, .. } => format!("{marker} {text}"),
        OutlineLine::Task { text, done, .. } => {
            format!("{} {text}", if *done { TICKED } else { UNTICKED })
        }
    }
}

/// A sheet's last results, each on the right of its line; a rule above a total.
fn calc_lines(ui: &mut Ui, rect: Rect, note: &DisplayNote, language: i18n::Language) {
    let p = palette(ui.ctx());
    let font = Font::mono(11.0, 1.3);
    let mut top = rect.top();
    let mut covered = egui::Rect::NOTHING;
    for line in &note.calc {
        let height = font.height() + if line.aggregate { 2.0 } else { 0.0 };
        if top + height > rect.bottom() + 0.5 {
            break;
        }
        if line.aggregate {
            ui.painter()
                .hline(rect.x_range(), top + 0.5, Stroke::new(1.0, p.line));
            top += 2.0;
        }
        let written = calc::value(&line.value, line.places, language);
        let result = if written.approx {
            format!("≈ {}", written.text)
        } else {
            written.text
        };
        let weight = if line.aggregate { 700.0 } else { 400.0 };
        let result = galley(
            ui,
            &result,
            &Font {
                weight,
                ..Font::mono(11.0, 1.3)
            },
            p.text_0,
            f32::INFINITY,
            1,
        );
        let room = (rect.width() - result.size().x - 10.0).max(0.0);
        let expression = galley(ui, &line.expression, &font, p.text_2, room, 1);
        let row = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), font.height()));
        covered = covered.union(row);
        ui.painter()
            .galley(row.min, expression, Color32::TRANSPARENT);
        ui.painter().galley(
            pos2(row.right() - result.size().x, top),
            result,
            Color32::TRANSPARENT,
        );
        top += font.height();
    }
    if covered.is_positive() {
        let words: Vec<&str> = note
            .calc
            .iter()
            .map(|line| line.expression.as_str())
            .collect();
        label(ui, covered, &words.join("\n"), "note-card-calc");
    }
}

/// The progress over the list, then two items: what is left to do first, the item a search
/// found always. Each item is ticked from the card.
fn checklist(
    ui: &mut Ui,
    rect: Rect,
    note: &DisplayNote,
    language: i18n::Language,
    actions: &mut Actions,
) {
    let p = palette(ui.ctx());
    let items = &note.items;
    let done = items.iter().filter(|item| item.done).count();
    let seats = seats(note);
    let font = Font::mono(9.5, 1.0);
    let count = i18n::translate(
        language,
        keys::notes::CHECKLIST_PROGRESS,
        &[
            ("done", u32::try_from(done).unwrap_or(u32::MAX).into()),
            (
                "total",
                u32::try_from(items.len()).unwrap_or(u32::MAX).into(),
            ),
        ],
    );
    let count = galley(ui, &count, &font, p.text_2, f32::INFINITY, 1);
    let hidden = items.len() - seats.len();
    let more = (hidden > 0).then(|| {
        let words = i18n::translate(
            language,
            keys::notes::CHECKLIST_MORE,
            &[("count", u32::try_from(hidden).unwrap_or(u32::MAX).into())],
        );
        galley(
            ui,
            &words,
            &Font::proportional(9.5, 400.0, 1.0),
            p.text_2,
            f32::INFINITY,
            1,
        )
    });
    let row = rect.top() + 6.0;
    let mut right = rect.right();
    if let Some(more) = more {
        right -= more.size().x;
        let words = more.text().to_string();
        text_at(
            ui,
            pos2(right, row - more.size().y / 2.0),
            more,
            &words,
            "note-card-more",
        );
        right -= 8.0;
    }
    right -= count.size().x;
    let words = count.text().to_string();
    text_at(
        ui,
        pos2(right, row - count.size().y / 2.0),
        count,
        &words,
        "note-card-progress",
    );
    let track = Rect::from_min_max(pos2(rect.left(), row - 2.5), pos2(right - 8.0, row + 2.5));
    ui.painter()
        .rect_filled(track, CornerRadius::same(3), p.bg_3);
    if !items.is_empty() {
        #[expect(clippy::cast_precision_loss, reason = "a list's length")]
        let share = done as f32 / items.len() as f32;
        let fill = Rect::from_min_size(track.min, vec2(track.width() * share, track.height()));
        ui.painter()
            .rect_filled(fill, CornerRadius::same(3), p.amber_fill);
    }

    let mut top = rect.top() + 12.0 + 4.0;
    for at in seats {
        if top + 24.0 > rect.bottom() + 0.5 {
            break;
        }
        let row = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), 24.0));
        if item(ui, row, &items[at], language) {
            let mut ticked = items.clone();
            ticked[at].done = !ticked[at].done;
            actions.push(Action::Notes(crate::notes::Action::SetItems {
                id: note.id.clone(),
                items: ticked,
            }));
        }
        top += 24.0;
    }
}

/// Which items a card shows, in the note's order: the one a search found, then the first
/// ones left to do, then the last ones — the top of a finished list says the least.
fn seats(note: &DisplayNote) -> Vec<usize> {
    let items = &note.items;
    let mut seats = Vec::new();
    if let Some(hit) = note
        .search_hit
        .as_ref()
        .filter(|hit| hit.field == SearchField::Item)
    {
        let needle = hit.excerpt.strip_suffix('…').unwrap_or(&hit.excerpt);
        if let Some(at) = items.iter().position(|item| item.text.starts_with(needle)) {
            seats.push(at);
        }
    }
    for (at, item) in items.iter().enumerate() {
        if seats.len() >= MAX_VISIBLE_ITEMS {
            break;
        }
        if !item.done && !seats.contains(&at) {
            seats.push(at);
        }
    }
    for at in (0..items.len()).rev() {
        if seats.len() >= MAX_VISIBLE_ITEMS {
            break;
        }
        if !seats.contains(&at) {
            seats.push(at);
        }
    }
    seats.sort_unstable();
    seats
}

/// One item, a checkbox of its own. `true` when it was clicked.
fn item(ui: &mut Ui, row: Rect, item: &ChecklistItem, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    let name = i18n::translate(
        language,
        keys::notes::CHECKLIST_TOGGLE,
        &[("text", item.text.as_str().into())],
    );
    let mut row_ui = ui.new_child(UiBuilder::new().max_rect(row));
    let done = item.done;
    let response = interact(
        &mut row_ui,
        row.size(),
        Sense::click(),
        "note-card-item",
        || WidgetInfo::selected(Role::CheckBox, true, done, &name),
        CornerRadius::same(4),
    );
    let ink = if response.hovered() {
        p.text_1
    } else {
        p.text_2
    };
    let box_ink = if done { p.amber_text } else { ink };
    let left = row.left() + 10.0 + 6.0;
    let mut format = Font::proportional(11.0, 400.0, 1.5).format(if done {
        ink.gamma_multiply(0.6)
    } else {
        ink
    });
    if done {
        format.strikethrough = Stroke::new(1.0, ink.gamma_multiply(0.6));
    }
    let mut job = egui::text::LayoutJob::single_section(item.text.clone(), format);
    job.wrap = egui::text::TextWrapping {
        max_width: (row.right() - left).max(1.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let text = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let middle = row.center().y;
    let tick = Rect::from_center_size(pos2(row.left() + 5.0, middle), vec2(10.0, 10.0));
    tick_box(ui, tick, done, box_ink);
    ui.painter().galley(
        pos2(left, middle - text.size().y / 2.0),
        text,
        Color32::TRANSPARENT,
    );
    response.clicked()
}

/// The picture, whole in the card's room: a sketch, not a crop of one. Asked for the first time
/// the card is drawn.
fn drawing(
    ui: &mut Ui,
    rect: Rect,
    note: &DisplayNote,
    context: &Context<'_>,
    actions: &mut Actions,
) {
    let stamp = Stamp {
        id: note.id.clone(),
        updated_at: note.updated_at,
    };
    match context.thumbnails.get(&stamp) {
        Thumbnail::Unknown => actions.push(Action::Notes(crate::notes::Action::Thumbnail(stamp))),
        Thumbnail::Waiting => {}
        Thumbnail::Missing => {
            let words = i18n::translate(context.language, keys::drawing::NO_PICTURE, &[]);
            prose(ui, rect, &words, false, "note-card-drawing-empty");
        }
        Thumbnail::Shown(texture) => {
            let size = texture.size_vec2();
            let scale = (rect.width() / size.x).min(rect.height() / size.y).min(1.0);
            let shown = Rect::from_min_size(rect.min, size * scale);
            let response = ui.allocate_rect(shown, Sense::hover());
            response.widget_info(|| WidgetInfo::labeled(Role::Image, true, ""));
            address(ui, &response, "note-card-drawing");
            egui::Image::new((texture.id(), shown.size()))
                .corner_radius(4)
                .paint_at(ui, shown);
        }
    }
}
