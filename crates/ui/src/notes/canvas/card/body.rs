//! What a card shows of its note, by its kind, in whole lines and no more than the library's
//! count: a snippet's first lines, a Note's outline, a list's tasks (one fewer than the lines, a
//! task's row being taller), a sheet's last results, a drawing's picture — or the line a search
//! found, when that is not code.

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
use crate::services::highlight;
use crate::state::drawing_pictures::{Stamp, Thumbnail};
use crate::theme::palette;
use crate::widgets::{address, interact};

/// A task's row: a box to tick needs it.
const ITEM_HEIGHT: f32 = 24.0;
/// A task's marks, as the screen reader is told them; drawn as boxes.
const TICKED: &str = "☑";
const UNTICKED: &str = "☐";

pub(super) fn show(
    ui: &mut Ui,
    rect: Rect,
    note: &DisplayNote,
    (context, lines): (&Context<'_>, usize),
    actions: &mut Actions,
) {
    let hit = note.search_hit.as_ref();
    match note.kind {
        NoteKind::Checklist => {
            let rows = lines.saturating_sub(1).max(1);
            checklist(ui, rect, (note, rows), context.language, actions);
        }
        NoteKind::Drawing if hit.is_none() => drawing(ui, rect, note, context, actions),
        NoteKind::Calc if hit.is_none() && !note.calc.is_empty() => {
            calc_lines(ui, rect, (note, lines), context.calc);
        }
        NoteKind::Note if hit.is_none() => outline(ui, rect, &note.outline, lines),
        NoteKind::Snippet if hit.is_none_or(|hit| hit.field == SearchField::Body) => {
            let shown = hit.map_or(note.content.as_str(), |hit| hit.excerpt.as_str());
            let syntax = crate::notes::language::syntax(note.language);
            code(ui, rect, (shown, lines), syntax);
        }
        _ => {
            let tag = hit.is_some_and(|hit| hit.field == SearchField::Tag);
            let words = hit.map_or(note.content.as_str(), |hit| hit.excerpt.as_str());
            prose(ui, rect, (words, lines), tag, "note-card-hit");
        }
    }
}

/// Monospaced, a line a row, cut at the card's edge rather than wrapped; coloured by the
/// grammar of its language, plain in the card's grey where it has none, with no frame. Its
/// `{{fields}}` are marked as the editor marks them.
fn code(ui: &mut Ui, rect: Rect, (content, lines): (&str, usize), syntax: Option<&str>) {
    let font = Font::mono(11.0, 16.0 / 11.0);
    label(ui, rect, content, "note-card-snippet");
    let painter = ui.painter().with_clip_rect(rect);
    let coloured = syntax.map(|syntax| highlight::short(ui.ctx(), syntax, content));
    let mut top = rect.top();
    for (at, line) in content.lines().take(lines).enumerate() {
        if top + font.height() > rect.bottom() + 0.5 {
            break;
        }
        let tokens = coloured.as_ref().and_then(|lines| lines.get(at));
        let job = code_line(ui, line, tokens, &font);
        let shown = ui.fonts_mut(|fonts| fonts.layout_job(job));
        painter.galley(pos2(rect.left(), top), shown, Color32::TRANSPARENT);
        top += font.height();
    }
}

/// One line's runs: each character's colour from its token, a field's over it.
fn code_line(
    ui: &Ui,
    line: &str,
    tokens: Option<&text::highlight::Line>,
    font: &Font,
) -> egui::text::LayoutJob {
    let p = palette(ui.ctx());
    let plain = (p.text_2, false, false);
    let mut styles = vec![plain; line.chars().count()];
    for (range, token) in tokens.into_iter().flatten() {
        let (colour, italics) = highlight::colour(p, *token);
        for style in styles.iter_mut().take(range.end).skip(range.start) {
            *style = (colour, italics, false);
        }
    }
    for range in crate::api::notes::field_ranges(line) {
        for style in styles.iter_mut().take(range.end).skip(range.start) {
            *style = (p.amber_text, false, true);
        }
    }
    let mut job = egui::text::LayoutJob::default();
    let mut run = String::new();
    let mut current = styles.first().copied().unwrap_or(plain);
    let flush = |job: &mut egui::text::LayoutJob, run: &mut String, (colour, italics, field)| {
        if run.is_empty() {
            return;
        }
        let mut format = font.format(colour);
        format.italics = italics;
        if field {
            format.background = p.amber_fill.gamma_multiply(0.14);
        }
        job.append(run, 0.0, format);
        run.clear();
    };
    for (c, style) in line.chars().zip(styles) {
        if style != current {
            flush(&mut job, &mut run, current);
            current = style;
        }
        run.push(c);
    }
    flush(&mut job, &mut run, current);
    job
}

/// The line a tag, an item or a Note's text put on the results: prose, never painted as code.
fn prose(ui: &mut Ui, rect: Rect, (words, lines): (&str, usize), tag: bool, author_id: &str) {
    let p = palette(ui.ctx());
    let font = Font::proportional(11.5, 400.0, 17.0 / 11.5);
    let rows = rows_in(rect.height(), font.height()).min(lines);
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
fn outline(ui: &mut Ui, rect: Rect, lines: &[OutlineLine], count: usize) {
    let p = palette(ui.ctx());
    let lines = &lines[..lines.len().min(count)];
    let words: Vec<String> = lines.iter().map(plain).collect();
    label(ui, rect, &words.join("\n"), "note-card-outline");
    let mut top = rect.top();
    for line in lines {
        let (font, colour, marker, depth, text) = match line {
            OutlineLine::Heading { text } => (
                Font::proportional(12.0, 600.0, 17.0 / 12.0),
                p.text_0,
                None,
                0,
                text,
            ),
            OutlineLine::Text { text } => (
                Font::proportional(12.0, 400.0, 17.0 / 12.0),
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
                Font::proportional(12.0, 400.0, 17.0 / 12.0),
                p.text_1,
                Some(marker.clone()),
                *depth,
                text,
            ),
            OutlineLine::Task { text, depth, done } => {
                let marker = if *done { TICKED } else { UNTICKED };
                (
                    Font::proportional(12.0, 400.0, 17.0 / 12.0),
                    p.text_1,
                    Some(marker.to_string()),
                    *depth,
                    text,
                )
            }
            OutlineLine::Code { text } => (Font::mono(11.0, 17.0 / 11.0), p.text_2, None, 0, text),
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
        top += font.height();
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
fn calc_lines(ui: &mut Ui, rect: Rect, (note, lines): (&DisplayNote, usize), format: calc::Format) {
    let p = palette(ui.ctx());
    let font = Font::mono(11.0, 16.0 / 11.0);
    let mut top = rect.top();
    let mut covered = egui::Rect::NOTHING;
    for line in note.calc.iter().take(lines) {
        let height = font.height() + if line.aggregate { 2.0 } else { 0.0 };
        if top + height > rect.bottom() + 0.5 {
            break;
        }
        if line.aggregate {
            ui.painter()
                .hline(rect.x_range(), top + 0.5, Stroke::new(1.0, p.line));
            top += 2.0;
        }
        let written = calc::value(&line.value, line.places, format);
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
                ..Font::mono(11.0, 16.0 / 11.0)
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

/// As many tasks as the card has rows for: what is left to do first, the task a search found
/// always. Each one is ticked from the card; how far along the list is sits beside its title.
fn checklist(
    ui: &mut Ui,
    rect: Rect,
    (note, rows): (&DisplayNote, usize),
    language: i18n::Language,
    actions: &mut Actions,
) {
    let items = &note.items;
    let mut top = rect.top();
    for at in seats(note, rows) {
        if top + ITEM_HEIGHT > rect.bottom() + 0.5 {
            break;
        }
        let row = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), ITEM_HEIGHT));
        if item(ui, row, &items[at], language) {
            let mut ticked = items.clone();
            ticked[at].done = !ticked[at].done;
            actions.push(Action::Notes(crate::notes::Action::SetItems {
                id: note.id.clone(),
                items: ticked,
            }));
        }
        top += ITEM_HEIGHT;
    }
}

/// Which items a card shows, in the note's order: the one a search found, then the first
/// ones left to do, then the last ones — the top of a finished list says the least.
fn seats(note: &DisplayNote, count: usize) -> Vec<usize> {
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
        if seats.len() >= count {
            break;
        }
        if !item.done && !seats.contains(&at) {
            seats.push(at);
        }
    }
    for at in (0..items.len()).rev() {
        if seats.len() >= count {
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
            prose(ui, rect, (&words, 1), false, "note-card-drawing-empty");
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
