//! A page of the documentation drawn, block by block.

use egui::text::LayoutJob;
use egui::{
    Align, Color32, CornerRadius, FontFamily, FontId, Layout, Role, Sense, Stroke, StrokeKind,
    TextFormat, Ui, Vec2, WidgetInfo, pos2, vec2,
};
use engine::docs::model::{Block, Inline, Page, Shortcut, Target, Tone};
use i18n::keys;

use super::text::{self, Style, lay, paragraph};
use super::{Go, swatch};
use crate::api::help;
use crate::state::AppState;
use crate::theme::icons::Icon;
use crate::theme::palette;
use crate::theme::tokens::{font, gap, radius};
use crate::widgets::{self, Button, IconButton, Kbd};

const CELL: Vec2 = vec2(gap::GROUPS, gap::CONTROLS);

/// What the page asks: a place to go, or a code block to copy. `section` is scrolled to, once.
pub(super) fn show(
    ui: &mut Ui,
    state: &AppState,
    page: &'static Page,
    section: &mut Option<String>,
) -> Option<Go> {
    ui.spacing_mut().item_spacing.y = 0.0;
    title(ui, state, page);
    let mut asked = None;
    for block in &page.body {
        ui.add_space(gap_before(block));
        let went = draw(ui, state, block, section);
        asked = asked.or(went);
    }
    ui.add_space(gap::PAGE);
    asked.or(pager(ui, state, page))
}

fn gap_before(block: &Block) -> f32 {
    match block {
        Block::Heading { level: 2, .. } => gap::PAGE,
        Block::Heading { .. } => gap::PANEL,
        _ => gap::GROUPS,
    }
}

fn title(ui: &mut Ui, state: &AppState, page: &Page) {
    let p = palette(ui.ctx());
    heading(ui, &page.title, font::DISPLAY, "docs-title");
    ui.add_space(gap::CONTROLS);
    let style = Style {
        size: font::SUBTITLE,
        weight: 400.0,
        colour: p.text_1,
    };
    let summary = [Inline::Text(page.summary.clone())];
    let width = ui.available_width();
    let laid = lay(ui, &summary, style, width, &state.settings.bindings);
    let (rect, _) = ui.allocate_exact_size(vec2(width, laid.size().y), Sense::hover());
    laid.show(ui, rect.min);
}

fn heading(ui: &mut Ui, words: &str, size: f32, author_id: &str) -> egui::Response {
    let ink = palette(ui.ctx()).text_0;
    let width = ui.available_width();
    let laid = widgets::wrapped(ui, words, (size, 650.0), ink, width);
    let (rect, response) = ui.allocate_exact_size(vec2(width, laid.size().y), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Heading, true, words));
    widgets::address(ui, &response, author_id);
    laid.paint(ui, rect.min, ink);
    response
}

fn draw(
    ui: &mut Ui,
    state: &AppState,
    block: &'static Block,
    section: &mut Option<String>,
) -> Option<Go> {
    let p = palette(ui.ctx());
    let bindings = &state.settings.bindings;
    match block {
        Block::Paragraph(runs) => go(paragraph(ui, runs, Style::body(p), bindings)),
        Block::Heading { level, id, text } => {
            let size = if *level == 2 {
                font::HEADING
            } else {
                font::SUBTITLE
            };
            let response = heading(ui, text, size, "docs-section");
            if section.as_deref() == Some(id) {
                let seen = response.rect.expand2(vec2(0.0, gap::PANEL));
                ui.scroll_to_rect(seen, Some(Align::TOP));
                *section = None;
            }
            None
        }
        Block::List { ordered, items } => list(ui, state, *ordered, items),
        Block::Note { tone, text } => note(ui, state, *tone, text),
        Block::Code {
            title,
            language,
            text,
        } => code(ui, state, title, language, text),
        Block::Shortcuts { title, rows } => {
            framed(ui, title, |ui| shortcuts(ui, state, rows));
            None
        }
        Block::Table { title, head, rows } => {
            let mut asked = None;
            framed(ui, title, |ui| asked = table(ui, state, head, rows));
            asked
        }
        Block::Pages(ids) => pages(ui, state, ids),
    }
}

fn go(target: Option<&Target>) -> Option<Go> {
    match target? {
        Target::Address(address) => Some(Go::Address(address.clone())),
        Target::Page { id, section } => Some(Go::Page(super::entry(id)?, section.clone())),
    }
}

fn list(ui: &mut Ui, state: &AppState, ordered: bool, items: &[Vec<Inline>]) -> Option<Go> {
    const MARK: f32 = gap::PANEL;
    let p = palette(ui.ctx());
    let mut asked = None;
    for (at, item) in items.iter().enumerate() {
        if at > 0 {
            ui.add_space(gap::SNUG);
        }
        let style = Style::body(p);
        let width = ui.available_width() - MARK;
        let laid = lay(ui, item, style, width, &state.settings.bindings);
        let size = vec2(ui.available_width(), laid.size().y);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let line = rect.top() + style.size * 0.8;
        if ordered {
            let number = format!("{}.", at + 1);
            let font = FontId::proportional(style.size);
            let place = pos2(rect.left() + MARK - gap::SNUG, line);
            ui.painter()
                .text(place, egui::Align2::RIGHT_CENTER, number, font, p.text_2);
        } else {
            let centre = pos2(rect.left() + MARK / 2.0, line);
            ui.painter().circle_filled(centre, 2.5, p.text_2);
        }
        let went = go(laid.show(ui, rect.min + vec2(MARK, 0.0)));
        asked = asked.or(went);
    }
    asked
}

fn note(ui: &mut Ui, state: &AppState, tone: Tone, runs: &[Inline]) -> Option<Go> {
    let p = palette(ui.ctx());
    let (hue, icon, name) = match tone {
        Tone::Tip => (p.amber_text, Icon::Info, keys::docs::TIP),
        Tone::Warning => (p.red, Icon::TriangleAlert, keys::docs::WARNING),
    };
    let name = i18n::translate(state.language, name, &[]);
    let frame = egui::Frame::new()
        .fill(hue.gamma_multiply(0.08))
        .stroke(Stroke::new(gap::HAIRLINE, hue.gamma_multiply(0.4)))
        .corner_radius(CornerRadius::same(radius::MENU))
        .inner_margin(gap::margin(gap::GROUPS, gap::INSET));
    let drawn = frame.show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap::INSET;
            let (mark, said) = ui.allocate_exact_size(Vec2::splat(font::HEADING), Sense::hover());
            said.widget_info(|| WidgetInfo::labeled(Role::Image, true, &name));
            crate::theme::icons::paint(ui, mark, icon, hue);
            let style = Style {
                colour: p.text_0,
                ..Style::body(p)
            };
            paragraph(ui, runs, style, &state.settings.bindings)
        })
        .inner
    });
    go(drawn.inner)
}

/// A table's and a code block's frame, under its title.
fn framed(ui: &mut Ui, title: &str, body: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    egui::Frame::new()
        .fill(p.bg_0)
        .stroke(Stroke::new(gap::HAIRLINE, p.line_soft))
        .corner_radius(CornerRadius::same(radius::CARD))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            caption(ui, title);
            body(ui);
        });
}

fn caption(ui: &mut Ui, title: &str) {
    let ink = palette(ui.ctx()).text_0;
    let words = widgets::text(
        ui,
        title,
        font::READING,
        600.0,
        FontFamily::Proportional,
        ink,
    );
    let size = vec2(ui.available_width(), words.size().y + 2.0 * CELL.y);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Caption, true, title));
    ui.painter().galley(rect.min + CELL, words, ink);
}

fn rule(ui: &Ui, top: f32) {
    let p = palette(ui.ctx());
    let line = Stroke::new(gap::HAIRLINE, p.line_soft);
    ui.painter().hline(ui.max_rect().x_range(), top, line);
}

fn shortcuts(ui: &mut Ui, state: &AppState, rows: &[Shortcut]) {
    const KEYS: f32 = 170.0;
    let p = palette(ui.ctx());
    for row in rows {
        rule(ui, ui.cursor().top());
        let width = ui.available_width() - KEYS - 2.0 * CELL.x;
        let said = [Inline::Text(row.does.clone())];
        let laid = lay(ui, &said, Style::body(p), width, &state.settings.bindings);
        let height = laid.size().y.max(24.0) + 2.0 * gap::TIGHT;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let caps = rect.shrink2(vec2(CELL.x, 0.0));
        let mut cell = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(caps)
                .layout(Layout::left_to_right(Align::Center)),
        );
        cell.spacing_mut().item_spacing.x = gap::TIGHT;
        for key in text::held(&row.keys, &state.settings.bindings) {
            Kbd(&key).show(&mut cell);
        }
        let top = rect.center().y - laid.size().y / 2.0;
        laid.show(ui, pos2(rect.left() + CELL.x + KEYS, top));
    }
}

fn table(ui: &mut Ui, state: &AppState, head: &[String], rows: &[Vec<Vec<Inline>>]) -> Option<Go> {
    let p = palette(ui.ctx());
    let bindings = &state.settings.bindings;
    let titles: Vec<_> = head
        .iter()
        .map(|title| vec![Inline::Text(title.clone())])
        .collect();
    let named = Style {
        size: font::LABEL,
        weight: 600.0,
        colour: p.text_2,
    };
    let natural = |cells: &[Vec<Inline>], style| -> Vec<f32> {
        cells
            .iter()
            .map(|cell| lay(ui, cell, style, f32::INFINITY, bindings).size().x)
            .collect()
    };
    let mut wanted = natural(&titles, named);
    for row in rows {
        let widths = natural(row, Style::body(p));
        for (widest, width) in wanted.iter_mut().zip(widths) {
            *widest = widest.max(width);
        }
    }
    let widths = columns(&wanted, ui.available_width());

    line(ui, state, &titles, named, &widths);
    let mut asked = None;
    for row in rows {
        rule(ui, ui.cursor().top());
        let went = line(ui, state, row, Style::body(p), &widths);
        asked = asked.or(went);
    }
    asked
}

/// What each column of `wanted` gets of `room`: its own width, the last one the rest; the widest
/// give way where they do not all fit.
fn columns(wanted: &[f32], room: f32) -> Vec<f32> {
    let inside = room - count(wanted.len()) * 2.0 * CELL.x;
    let mut widths = wanted.to_vec();
    let mut over = widths.iter().sum::<f32>() - inside;
    while over > 0.5 {
        let widest = widths.iter().copied().fold(0.0, f32::max);
        let wide: Vec<_> = (0..widths.len())
            .filter(|at| widths[*at] > widest - 0.5)
            .collect();
        let next = widths
            .iter()
            .copied()
            .filter(|width| *width <= widest - 0.5)
            .fold(0.0, f32::max);
        let cut = (widest - next).min(over / count(wide.len()));
        if cut <= 0.0 {
            break;
        }
        for at in &wide {
            widths[*at] -= cut;
        }
        over -= cut * count(wide.len());
    }
    if let Some(last) = widths.last_mut() {
        *last += (-over).max(0.0);
    }
    widths
}

#[expect(clippy::cast_precision_loss, reason = "a few columns")]
fn count(columns: usize) -> f32 {
    columns as f32
}

fn line(
    ui: &mut Ui,
    state: &AppState,
    cells: &[Vec<Inline>],
    style: Style,
    widths: &[f32],
) -> Option<Go> {
    let laid: Vec<_> = cells
        .iter()
        .zip(widths)
        .map(|(cell, width)| lay(ui, cell, style, *width, &state.settings.bindings))
        .collect();
    let height = laid.iter().map(|cell| cell.size().y).fold(0.0, f32::max);
    let size = vec2(ui.available_width(), height + 2.0 * CELL.y);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let mut left = rect.left();
    let mut asked = None;
    for (cell, width) in laid.into_iter().zip(widths) {
        let went = go(cell.show(ui, pos2(left, rect.top()) + CELL));
        asked = asked.or(went);
        left += width + 2.0 * CELL.x;
    }
    asked
}

fn code(ui: &mut Ui, state: &AppState, title: &str, language: &str, source: &str) -> Option<Go> {
    let p = palette(ui.ctx());
    let mut asked = None;
    framed(ui, title, |ui| {
        let top = ui.max_rect().right_top() + vec2(-CELL.x, gap::SNUG);
        let bar =
            egui::Rect::from_min_max(pos2(ui.max_rect().left(), top.y), top + vec2(0.0, 24.0));
        let mut end = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar)
                .layout(Layout::right_to_left(Align::Center)),
        );
        let label = i18n::translate(state.language, keys::docs::COPY, &[]);
        if IconButton::new(Icon::Copy, &label, "docs-copy")
            .show(&mut end)
            .clicked()
        {
            asked = Some(Go::Copy(source.to_owned()));
        }
        if !language.is_empty() {
            widgets::hue_badge(&mut end, language, p.green);
        }

        rule(ui, ui.cursor().top());
        let job = coloured(ui, language, source);
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        let size = vec2(ui.available_width(), galley.size().y + 2.0 * CELL.y);
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(Role::Code, true, source));
        ui.painter().galley(rect.min + CELL, galley, p.text_0);
    });
    asked
}

fn coloured(ui: &Ui, language: &str, source: &str) -> LayoutJob {
    let p = palette(ui.ctx());
    let font = FontId::monospace(font::LABEL);
    let mut job = LayoutJob::default();
    job.wrap.max_width = ui.available_width() - 2.0 * CELL.x;
    let syntax = widgets::syntax_of(language);
    let lines = syntax.map(|syntax| crate::services::highlight::short(ui.ctx(), syntax, source));
    for (at, line) in source.split('\n').enumerate() {
        if at > 0 {
            job.append("\n", 0.0, TextFormat::simple(font.clone(), p.text_0));
        }
        let tokens = lines.as_ref().and_then(|lines| lines.get(at));
        widgets::append_coloured(&mut job, p, line, tokens, &font);
    }
    job
}

/// The overview's: a card per page, its name and what it is about.
fn pages(ui: &mut Ui, state: &AppState, ids: &[String]) -> Option<Go> {
    let mut asked = None;
    let width = (ui.available_width() - gap::GROUPS) / 2.0;
    for pair in ids.chunks(2) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap::GROUPS;
            for id in pair {
                if let Some(id) = super::entry(id)
                    && card(ui, state, id, width)
                {
                    asked = Some(Go::Page(id, None));
                }
            }
        });
        ui.add_space(gap::GROUPS);
    }
    asked
}

fn card(ui: &mut Ui, state: &AppState, id: &'static str, width: f32) -> bool {
    /// Three lines of what the page is about.
    const HEIGHT: f32 = 96.0;
    const SQUARE: f32 = 9.0;
    let p = palette(ui.ctx());
    let Some(page) = help::docs_page(state.language, id) else {
        return false;
    };
    let corner = CornerRadius::same(radius::CARD);
    let response = widgets::interact(
        ui,
        vec2(width, HEIGHT),
        Sense::click(),
        &format!("docs-card-{id}"),
        || WidgetInfo::labeled(Role::Link, true, &page.title),
        corner,
    );
    let rect = response.rect;
    let edge = if response.hovered() {
        p.line
    } else {
        p.line_soft
    };
    ui.painter().rect(
        rect,
        corner,
        p.bg_2,
        Stroke::new(gap::HAIRLINE, edge),
        StrokeKind::Inside,
    );
    let square = egui::Rect::from_min_size(
        rect.min + vec2(gap::GROUPS, gap::GROUPS + 4.0),
        Vec2::splat(SQUARE),
    );
    ui.painter()
        .rect_filled(square, CornerRadius::same(radius::MARK), swatch(p, id));
    let left = square.right() + gap::CONTROLS;
    let inside = rect.right() - left - gap::GROUPS;
    let name = widgets::one_line(ui, &page.title, (font::READING, 600.0), p.text_0, inside);
    let under = rect.top() + gap::GROUPS + name.size().y + gap::TIGHT;
    ui.painter()
        .galley(pos2(left, rect.top() + gap::GROUPS), name, p.text_0);
    about(ui, &page.summary, (pos2(left, under), inside), p.text_1);
    response.clicked()
}

/// Three lines at most, cut with an ellipsis.
fn about(ui: &Ui, words: &str, (at, width): (egui::Pos2, f32), ink: Color32) {
    let font = FontId::proportional(font::LABEL);
    let mut job = LayoutJob::simple(words.to_owned(), font, ink, width);
    job.wrap.max_rows = 3;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter().galley(at, galley, ink);
}

/// The pages before and after, in reading order.
fn pager(ui: &mut Ui, state: &AppState, page: &Page) -> Option<Go> {
    let entries: Vec<_> = help::docs_index().pages().collect();
    let at = entries.iter().position(|entry| entry.id == page.id)?;
    let name = |at: usize| {
        entries
            .get(at)
            .map(|entry| entry.name.of(state.language.code()))
    };
    let mut asked = None;
    ui.horizontal(|ui| {
        if let Some((before, label)) = at.checked_sub(1).and_then(|at| Some((at, name(at)?)))
            && Button::new(label, "docs-previous")
                .icon(Icon::ArrowLeft)
                .show(ui)
                .clicked()
        {
            asked = Some(Go::Page(entries[before].id.as_str(), None));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(label) = name(at + 1)
                && Button::new(label, "docs-next")
                    .icon(Icon::ArrowRight)
                    .show(ui)
                    .clicked()
            {
                asked = Some(Go::Page(entries[at + 1].id.as_str(), None));
            }
        });
    });
    asked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_that_fit_keep_their_width_and_the_last_takes_the_rest() {
        let widths = columns(&[50.0, 100.0], 50.0 + 200.0 + 4.0 * CELL.x);
        assert_eq!(widths, [50.0, 200.0]);
    }

    #[test]
    fn the_widest_column_gives_way_to_the_others() {
        let widths = columns(&[50.0, 400.0], 200.0 + 4.0 * CELL.x);
        assert!((widths[0] - 50.0).abs() < 1.0, "{widths:?}");
        assert!((widths[1] - 150.0).abs() < 1.0, "{widths:?}");
        let widths = columns(&[300.0, 300.0, 40.0], 240.0 + 6.0 * CELL.x);
        assert!((widths[0] - 100.0).abs() < 1.0, "{widths:?}");
        assert!((widths[2] - 40.0).abs() < 1.0, "{widths:?}");
    }
}
