//! A formatted Note read, not edited (the reference's `createRichViewer`): each block laid out as
//! one `LayoutJob` — headings, emphasis, lists and tasks, quotes, tables, coloured code blocks,
//! links, rules — stacked in the reference's `.rich-text` rhythm. egui's galley cache keeps each
//! block's layout between frames. A link is answered to the caller, `http` and `https` alone.

use egui::text::LayoutJob;
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind, TextFormat,
    Ui, Vec2, pos2, vec2,
};
use richtext::{Block, Document, Inline, Item, ListKind, Marks};

use crate::theme::palette;
use crate::theme::tokens::Palette;

/// The body's size; headings and code are sized from it, as the reference's `em`s were.
const BODY: f32 = 14.0;
const LINE_HEIGHT: f32 = 1.65;
/// `> * + *`: 0.7em between blocks.
const GAP: f32 = 0.7;
const INDENT: f32 = 1.4;
const CODE_SIZE: f32 = 12.5;

/// What a frame of the reader was asked: a link clicked.
#[derive(Debug, Default)]
pub struct Read {
    pub link: Option<String>,
}

/// The document at `scale` (1 in a Note; Present's slides read larger).
pub fn rich_reader(ui: &mut Ui, document: &Document, scale: f32) -> Read {
    let mut read = Read::default();
    let size = BODY * scale;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        for (at, block) in document.blocks.iter().enumerate() {
            if at > 0 {
                ui.add_space(gap_before(block, size));
            }
            draw_block(ui, block, size, &mut read);
        }
    });
    read
}

fn gap_before(block: &Block, size: f32) -> f32 {
    match block {
        Block::Heading { level, .. } if *level <= 3 => 1.1 * heading_size(*level, size),
        _ => GAP * size,
    }
}

fn heading_size(level: u8, size: f32) -> f32 {
    match level {
        1 => size * 22.0 / BODY,
        2 => size * 18.0 / BODY,
        3 => size * 15.0 / BODY,
        _ => size,
    }
}

fn draw_block(ui: &mut Ui, block: &Block, size: f32, read: &mut Read) {
    let p = palette(ui.ctx());
    match block {
        Block::Paragraph(content) => paragraph(ui, content, Style::body(p, size), read),
        Block::Heading { level, content } => {
            let style = Style {
                size: heading_size(*level, size),
                weight: 600.0,
                line_height: 1.3,
                ..Style::body(p, size)
            };
            paragraph(ui, content, style, read);
        }
        Block::List { kind, items } => list(ui, *kind, items, size, read),
        Block::Quote(blocks) => quote(ui, blocks, size, read),
        Block::CodeBlock { language, text } => code_block(ui, language, text, size),
        Block::Table(rows) => table(ui, rows, size),
        Block::Image { alt, src } => image(ui, alt, src, size),
        Block::Rule => {
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), size), Sense::hover());
            ui.painter()
                .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, p.line));
        }
    }
}

/// How a run of a block is written before its marks.
#[derive(Clone, Copy)]
struct Style {
    size: f32,
    weight: f32,
    line_height: f32,
    colour: Color32,
    /// A checked task's text: struck through and dimmed.
    struck: bool,
}

impl Style {
    fn body(p: &Palette, size: f32) -> Self {
        Self {
            size,
            weight: 400.0,
            line_height: LINE_HEIGHT,
            colour: p.text_0,
            struck: false,
        }
    }
}

/// The block's text as one job, and the link each `char` range points to.
fn job(
    p: &Palette,
    content: &[Inline],
    style: Style,
    width: f32,
) -> (LayoutJob, Vec<(std::ops::Range<usize>, String)>) {
    let mut job = LayoutJob::default();
    let mut links = Vec::new();
    let mut chars = 0;
    for inline in content {
        let (text, marks) = match inline {
            Inline::Text { text, marks } => (text.as_str(), marks),
            Inline::HardBreak => ("\n", &Marks::default()),
        };
        let format = format(p, marks, style);
        job.append(text, 0.0, format);
        let count = text.chars().count();
        if let Some(href) = &marks.link {
            links.push((chars..chars + count, href.clone()));
        }
        chars += count;
    }
    job.wrap.max_width = width;
    (job, links)
}

fn format(p: &Palette, marks: &Marks, style: Style) -> TextFormat {
    let (family, size) = if marks.code {
        (FontFamily::Monospace, style.size * 0.88)
    } else {
        (FontFamily::Proportional, style.size)
    };
    let mut colour = style.colour;
    if marks.strike || style.struck {
        colour = p.text_2;
    }
    if marks.link.is_some() {
        colour = p.amber_text;
    }
    let mut format = TextFormat {
        font_id: FontId::new(size, family),
        color: colour,
        italics: marks.italic,
        line_height: Some(style.size * style.line_height),
        background: if marks.code {
            p.bg_3
        } else {
            Color32::TRANSPARENT
        },
        ..TextFormat::default()
    };
    let weight = if marks.bold { 650.0 } else { style.weight };
    format.coords.push(*b"wght", weight);
    if marks.strike || style.struck {
        format.strikethrough = Stroke::new(1.0, colour);
    }
    if marks.link.is_some() {
        format.underline = Stroke::new(1.0, colour);
    }
    format
}

fn paragraph(ui: &mut Ui, content: &[Inline], style: Style, read: &mut Read) {
    let p = palette(ui.ctx());
    let (job, links) = job(p, content, style, ui.available_width());
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let sense = if links.is_empty() {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
    let plain: String = content
        .iter()
        .map(|inline| match inline {
            Inline::Text { text, .. } => text.as_str(),
            Inline::HardBreak => "\n",
        })
        .collect();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &plain));
    if !links.is_empty()
        && let Some(pointer) = response.hover_pos()
    {
        let at = galley.cursor_from_pos(pointer - rect.min).index.0;
        if let Some((_, href)) = links.iter().find(|(range, _)| range.contains(&at)) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            if response.clicked() && is_web(href) {
                read.link = Some(href.clone());
            }
        }
    }
    ui.painter().galley(rect.min, galley, style.colour);
}

/// `http` and `https` alone: a Note's link never opens a file or runs a scheme.
fn is_web(href: &str) -> bool {
    let lower = href.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn list(ui: &mut Ui, kind: ListKind, items: &[Item], size: f32, read: &mut Read) {
    let p = palette(ui.ctx());
    let indent = if kind == ListKind::Task {
        0.0
    } else {
        INDENT * size
    };
    for (at, item) in items.iter().enumerate() {
        if at > 0 {
            ui.add_space(0.2 * size);
        }
        ui.horizontal_top(|ui| {
            let marker_width = match kind {
                ListKind::Task => 14.0 + 8.0,
                _ => indent,
            };
            let line = size * LINE_HEIGHT;
            let (marker, _) = ui.allocate_exact_size(vec2(marker_width, line), Sense::hover());
            match kind {
                ListKind::Bullet => {
                    let centre = pos2(marker.right() - 0.6 * size, marker.center().y);
                    ui.painter()
                        .circle_filled(centre, 2.5 * size / BODY, p.text_1);
                }
                ListKind::Ordered { start } => {
                    let number = format!("{}.", start + u64::try_from(at).unwrap_or(0));
                    ui.painter().text(
                        pos2(marker.right() - 0.35 * size, marker.center().y),
                        Align2::RIGHT_CENTER,
                        number,
                        FontId::proportional(size),
                        p.text_1,
                    );
                }
                ListKind::Task => checkbox(ui, marker, item.checked, size),
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for (at, block) in item.blocks.iter().enumerate() {
                    if at > 0 {
                        ui.add_space(0.5 * size);
                    }
                    match block {
                        Block::Paragraph(content) if kind == ListKind::Task => {
                            let style = Style {
                                struck: item.checked,
                                ..Style::body(p, size)
                            };
                            paragraph(ui, content, style, read);
                        }
                        _ => draw_block(ui, block, size, read),
                    }
                }
            });
        });
    }
}

/// A read-only task box: amber when done, as the reference's checkbox.
fn checkbox(ui: &Ui, marker: Rect, checked: bool, size: f32) {
    let p = palette(ui.ctx());
    let edge = 14.0 * size / BODY;
    let rect = Rect::from_min_size(
        pos2(marker.left(), marker.center().y - edge / 2.0),
        Vec2::splat(edge),
    );
    let corner = CornerRadius::same(3);
    if checked {
        ui.painter().rect_filled(rect, corner, p.amber_fill);
        let tick = [
            pos2(rect.left() + edge * 0.25, rect.center().y),
            pos2(rect.left() + edge * 0.43, rect.bottom() - edge * 0.28),
            pos2(rect.right() - edge * 0.22, rect.top() + edge * 0.3),
        ];
        ui.painter()
            .line(tick.to_vec(), Stroke::new(1.6, p.amber_ink));
    } else {
        ui.painter()
            .rect_stroke(rect, corner, Stroke::new(1.0, p.line), StrokeKind::Inside);
    }
}

fn quote(ui: &mut Ui, blocks: &[Block], size: f32, read: &mut Read) {
    let p = palette(ui.ctx());
    let top = ui.cursor().top();
    ui.horizontal_top(|ui| {
        ui.add_space(14.0 + 3.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.add_space(2.0);
            for (at, block) in blocks.iter().enumerate() {
                if at > 0 {
                    ui.add_space(GAP * size);
                }
                match block {
                    Block::Paragraph(content) => {
                        let style = Style {
                            colour: p.text_1,
                            ..Style::body(p, size)
                        };
                        paragraph(ui, content, style, read);
                    }
                    _ => draw_block(ui, block, size, read),
                }
            }
            ui.add_space(2.0);
        });
    });
    let bottom = ui.cursor().top();
    let bar = Rect::from_min_max(
        pos2(ui.max_rect().left(), top),
        pos2(ui.max_rect().left() + 3.0, bottom),
    );
    ui.painter()
        .rect_filled(bar, CornerRadius::ZERO, p.amber_dim);
}

fn code_block(ui: &mut Ui, language: &str, text: &str, size: f32) {
    let p = palette(ui.ctx());
    let font = FontId::monospace(CODE_SIZE * size / BODY);
    let syntax = syntax_of(language);
    let lines = syntax.map(|syntax| crate::services::highlight::short(ui.ctx(), syntax, text));
    let mut job = LayoutJob::default();
    for (at, line) in text.trim_end_matches('\n').split('\n').enumerate() {
        if at > 0 {
            job.append("\n", 0.0, TextFormat::simple(font.clone(), p.text_0));
        }
        let tokens = lines.as_ref().and_then(|lines| lines.get(at));
        append_coloured(&mut job, p, line, tokens, &font);
    }
    job.wrap.max_width = f32::INFINITY;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let padding = vec2(14.0, 12.0);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        vec2(width, galley.size().y + 2.0 * padding.y),
        Sense::hover(),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Code, true, text));
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        p.bg_1,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    ui.painter()
        .with_clip_rect(rect.shrink(1.0))
        .galley(rect.min + padding, galley, p.text_0);
    if !language.is_empty() {
        ui.painter().text(
            pos2(rect.right() - 10.0, rect.top() + 6.0),
            Align2::RIGHT_TOP,
            language,
            FontId::monospace(10.0),
            p.text_2,
        );
    }
}

/// A fence's word: a language's tag as the editor writes it, or a shell's usual name.
fn syntax_of(fence: &str) -> Option<&'static str> {
    use engine::notes::language::Language;
    let lower = fence.to_ascii_lowercase();
    let language = match lower.as_str() {
        "bash" | "shell" | "zsh" => Some(Language::Sh),
        tag => tag.parse::<Language>().ok(),
    };
    language.and_then(crate::notes::language::syntax)
}

fn append_coloured(
    job: &mut LayoutJob,
    p: &Palette,
    line: &str,
    tokens: Option<&text::highlight::Line>,
    font: &FontId,
) {
    let Some(tokens) = tokens else {
        job.append(line, 0.0, TextFormat::simple(font.clone(), p.text_0));
        return;
    };
    let byte = |chars: usize| {
        line.char_indices()
            .nth(chars)
            .map_or(line.len(), |(at, _)| at)
    };
    let mut from = 0;
    for (range, token) in tokens {
        let (start, end) = (byte(range.start), byte(range.end));
        if start > from {
            job.append(
                &line[from..start],
                0.0,
                TextFormat::simple(font.clone(), p.text_0),
            );
        }
        let (colour, leans) = crate::services::highlight::colour(p, *token);
        let format = TextFormat {
            italics: leans,
            ..TextFormat::simple(font.clone(), colour)
        };
        job.append(&line[start..end.max(start)], 0.0, format);
        from = end.max(start);
    }
    if from < line.len() {
        job.append(
            &line[from..],
            0.0,
            TextFormat::simple(font.clone(), p.text_0),
        );
    }
}

fn table(ui: &mut Ui, rows: &[Vec<richtext::Cell>], size: f32) {
    let p = palette(ui.ctx());
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    if columns == 0 {
        return;
    }
    ui.add_space(0.1 * size);
    let width = ui.available_width();
    let column = width / f32::from(u16::try_from(columns).unwrap_or(u16::MAX));
    let padding = vec2(10.0, 6.0);
    for (at, row) in rows.iter().enumerate() {
        let header = at == 0;
        let style = Style {
            weight: if header { 600.0 } else { 400.0 },
            ..Style::body(p, size)
        };
        let jobs: Vec<_> = (0..columns)
            .map(|index| {
                let cell = row.get(index).map_or(&[][..], Vec::as_slice);
                let (job, _) = job(p, cell, style, column - 2.0 * padding.x);
                ui.fonts_mut(|fonts| fonts.layout_job(job))
            })
            .collect();
        let height = jobs
            .iter()
            .map(|galley| galley.size().y)
            .fold(size * LINE_HEIGHT, f32::max)
            + 2.0 * padding.y;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        for (index, galley) in jobs.into_iter().enumerate() {
            let left = rect.left() + column * f32::from(u16::try_from(index).unwrap_or(0));
            let cell = Rect::from_min_size(pos2(left, rect.top()), vec2(column, height));
            if header {
                ui.painter().rect_filled(cell, CornerRadius::ZERO, p.bg_2);
            }
            ui.painter().rect_stroke(
                cell,
                CornerRadius::ZERO,
                Stroke::new(1.0, p.line),
                StrokeKind::Middle,
            );
            ui.painter().galley(cell.min + padding, galley, p.text_0);
        }
    }
}

/// A drawing embedded by `devnotes:drawing/<id>` is drawn with the editor (#1215); here its name.
fn image(ui: &mut Ui, alt: &str, src: &str, size: f32) {
    let p = palette(ui.ctx());
    let words = if alt.is_empty() { src } else { alt };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 3.0 * size), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, words));
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        p.bg_1,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        words,
        FontId::proportional(12.0),
        p.text_2,
    );
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::{NodeT, Queryable};

    use super::*;

    fn shown(markdown: &'static str) -> egui_kittest::Harness<'static, Option<String>> {
        let document = richtext::read(markdown);
        let mut harness = crate::testing::harness()
            .with_size(vec2(700.0, 900.0))
            .build_ui_state(
                move |ui, link: &mut Option<String>| {
                    if let Some(href) = rich_reader(ui, &document, 1.0).link {
                        *link = Some(href);
                    }
                },
                None,
            );
        harness.run();
        harness
    }

    fn says(harness: &egui_kittest::Harness<'_, Option<String>>, words: &str) -> bool {
        harness
            .query_all_by(|node| {
                node.label()
                    .or_else(|| node.value())
                    .is_some_and(|said| said.contains(words))
            })
            .next()
            .is_some()
    }

    #[test]
    fn every_sample_note_reads_in_both_languages() {
        let fr = i18n::translate(
            i18n::Language::Fr,
            i18n::keys::notes::samples::release::CONTENT,
            &[],
        );
        let en = i18n::translate(
            i18n::Language::En,
            i18n::keys::notes::samples::release::CONTENT,
            &[],
        );
        for content in [fr, en] {
            let document = richtext::read(&content);
            let mut harness = crate::testing::harness()
                .with_size(vec2(700.0, 900.0))
                .build_ui(move |ui| {
                    rich_reader(ui, &document, 1.0);
                });
            harness.run();
            let code = harness.get_by(|node| node.role() == egui::accesskit::Role::Code);
            assert!(
                code.accesskit_node()
                    .label()
                    .unwrap_or_default()
                    .starts_with("kubectl rollout")
            );
        }
    }

    #[test]
    fn headings_lists_tasks_quotes_and_tables_are_read() {
        let harness = shown(
            "# Titre\n\n- un\n  - deux\n\n1. premier\n\n- [x] fait\n\n> cité\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n\n---",
        );
        for words in ["Titre", "un", "deux", "premier", "fait", "cité"] {
            assert!(says(&harness, words), "{words} is read");
        }
    }

    #[test]
    fn a_web_link_is_answered_and_another_scheme_is_not() {
        let mut harness = shown("[le site](https://exemple.fr)");
        harness.get_by_label("le site").click();
        harness.run();
        assert_eq!(harness.state().as_deref(), Some("https://exemple.fr"));

        let mut harness = shown("[un fichier](file:///etc/passwd)");
        harness.get_by_label("un fichier").click();
        harness.run();
        assert!(harness.state().is_none(), "never a file");
    }
}
