//! A formatted Note drawn (the reference's `.rich-text`): each block laid out as one `LayoutJob` —
//! headings, emphasis, lists and tasks, quotes, tables, coloured code blocks, links, rules —
//! stacked in its rhythm; egui's galley cache keeps each block's layout between frames. Read, it
//! answers a link clicked, `http` and `https` alone (the reference's `createRichViewer`). Edited,
//! it paints the selection under the text and says where each leaf was laid out, for the editor
//! to place the caret and read a click. A `mermaid` block is drawn under its code, which folds
//! to its first line away from the caret.

use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Galley, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextFormat, Ui, Vec2, pos2, vec2,
};
use richtext::{Block, Document, Inline, Item, ListKind, Marks, Selection};

use i18n::keys;

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

/// The document read at `scale` (1 in a Note; Present's slides read larger).
pub fn rich_reader(ui: &mut Ui, document: &Document, scale: f32) -> Read {
    let painted = RichView::new(document).scale(scale).show(ui);
    Read { link: painted.link }
}

/// A leaf as this frame laid it out.
pub struct Laid {
    pub path: Vec<usize>,
    pub origin: Pos2,
    pub galley: Arc<Galley>,
}

/// What a frame drew: each leaf, each task's box, and a link clicked.
#[derive(Default)]
pub struct Painted {
    pub leaves: Vec<Laid>,
    /// A task's box clicked: the path of its list and the item's index.
    pub ticked: Option<(Vec<usize>, usize)>,
    /// Read: clicked; edited: clicked with Ctrl.
    pub link: Option<String>,
    /// The drawings whose picture was never asked for.
    pub wanted: Vec<String>,
    /// The diagrams not drawn yet.
    pub sketches: Vec<Sketch>,
    /// A diagram clicked: the path of its code.
    pub diagram: Option<Vec<usize>>,
}

/// How a Note names a drawing it embeds: an image whose source is the drawing's id.
pub const DRAWING_SOURCE: &str = "devnotes:drawing/";

/// What stands where a Note embeds a drawing.
pub enum Embedded {
    Waiting,
    /// The drawing is gone, or has no picture yet: said in words.
    Missing,
    Shown(egui::TextureHandle),
}

/// A drawing's picture by its id; `None` when it was never asked for.
pub type Pictures<'a> = &'a dyn Fn(&str) -> Option<Embedded>;

/// A `mermaid` block's drawing, or why there is none.
#[derive(Clone)]
pub enum Diagram {
    Drawn(egui::TextureHandle),
    Failed { line: Option<usize>, detail: String },
    Unsupported,
}

/// A diagram by its source; `None` while it is not drawn.
pub type Diagrams<'a> = &'a dyn Fn(&str) -> Option<Diagram>;

/// A diagram to draw: its source, and whether an earlier drawing stands in its place meanwhile.
pub struct Sketch {
    pub source: String,
    pub stale: bool,
}

/// A match the find bar lights: the leaf it is in and its `char` range there.
pub type Found = (Vec<usize>, std::ops::Range<usize>);

pub struct RichView<'a> {
    document: &'a Document,
    scale: f32,
    found: &'a [Found],
    /// The embedded drawings' pictures, and what a missing one says.
    pictures: Option<(Pictures<'a>, &'a str)>,
    diagrams: Option<(Diagrams<'a>, i18n::Language)>,
    /// Edited: the selection painted, links followed with Ctrl alone.
    selection: Option<&'a Selection>,
}

/// What every block of a frame is drawn with.
struct Frame<'a> {
    size: f32,
    pictures: Option<(Pictures<'a>, &'a str)>,
    diagrams: Option<(Diagrams<'a>, i18n::Language)>,
    /// Where each diagram's last drawing is kept, by its rank in the Note.
    places: egui::Id,
    rank: usize,
    selection: Option<&'a Selection>,
    found: &'a [Found],
    painted: Painted,
}

impl<'a> RichView<'a> {
    pub fn new(document: &'a Document) -> Self {
        Self {
            document,
            scale: 1.0,
            found: &[],
            pictures: None,
            diagrams: None,
            selection: None,
        }
    }

    /// `mermaid` blocks drawn; without it, they are code.
    #[must_use]
    pub fn diagrams(mut self, diagrams: Diagrams<'a>, language: i18n::Language) -> Self {
        self.diagrams = Some((diagrams, language));
        self
    }

    /// The pictures of the drawings the Note embeds; without them, each one's name.
    #[must_use]
    pub fn pictures(mut self, pictures: Pictures<'a>, missing: &'a str) -> Self {
        self.pictures = Some((pictures, missing));
        self
    }

    /// The find bar's matches, lit under the text.
    #[must_use]
    pub fn found(mut self, found: &'a [Found]) -> Self {
        self.found = found;
        self
    }

    #[must_use]
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    #[must_use]
    pub fn editing(mut self, selection: &'a Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Painted {
        let mut frame = Frame {
            size: BODY * self.scale,
            pictures: self.pictures,
            diagrams: self.diagrams,
            places: ui.unique_id().with("rich-diagrams"),
            rank: 0,
            selection: self.selection,
            found: self.found,
            painted: Painted::default(),
        };
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            blocks(ui, &self.document.blocks, &mut Vec::new(), &mut frame);
        });
        frame.painted
    }
}

fn blocks(ui: &mut Ui, blocks: &[Block], path: &mut Vec<usize>, frame: &mut Frame<'_>) {
    for (at, block) in blocks.iter().enumerate() {
        if at > 0 {
            ui.add_space(gap_before(block, frame.size));
        }
        path.push(at);
        draw_block(ui, block, path, frame);
        path.pop();
    }
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

fn draw_block(ui: &mut Ui, block: &Block, path: &mut Vec<usize>, frame: &mut Frame<'_>) {
    let p = palette(ui.ctx());
    let size = frame.size;
    match block {
        Block::Paragraph(content) => paragraph(ui, content, Style::body(p, size), path, frame),
        Block::Heading { level, content } => {
            let style = Style {
                size: heading_size(*level, size),
                weight: 600.0,
                line_height: 1.3,
                ..Style::body(p, size)
            };
            paragraph(ui, content, style, path, frame);
        }
        Block::List { kind, items } => list(ui, *kind, items, path, frame),
        Block::Quote(inner) => quote(ui, inner, path, frame),
        Block::CodeBlock { language, text } => code_block(ui, language, text, path, frame),
        Block::Table(rows) => table(ui, rows, path, frame),
        Block::Image { alt, src } => image(ui, alt, src, frame),
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

type Links = Vec<(std::ops::Range<usize>, String)>;

/// The block's text as one job, and the link each `char` range points to.
fn job(p: &Palette, content: &[Inline], style: Style, width: f32) -> (LayoutJob, Links) {
    let mut job = LayoutJob::default();
    let mut links = Vec::new();
    let mut chars = 0;
    for inline in content {
        let (text, marks) = match inline {
            Inline::Text { text, marks } => (text.as_str(), marks),
            Inline::HardBreak => ("\n", &Marks::default()),
        };
        job.append(text, 0.0, format(p, marks, style));
        let count = text.chars().count();
        if let Some(href) = &marks.link {
            links.push((chars..chars + count, href.clone()));
        }
        chars += count;
    }
    if content.is_empty() {
        // An empty block keeps a line's height, for the caret to stand in.
        job.append("", 0.0, format(p, &Marks::default(), style));
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

fn paragraph(ui: &mut Ui, content: &[Inline], style: Style, path: &[usize], frame: &mut Frame<'_>) {
    let p = palette(ui.ctx());
    let (job, links) = job(p, content, style, ui.available_width());
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let reading = frame.selection.is_none();
    let sense = if reading && !links.is_empty() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
    if reading {
        let plain = richtext::inlines::plain(content);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &plain));
    }
    if let Some(pointer) = ui.ctx().pointer_hover_pos().filter(|at| rect.contains(*at)) {
        let at = galley.cursor_from_pos(pointer - rect.min).index.0;
        if let Some((_, href)) = links.iter().find(|(range, _)| range.contains(&at)) {
            let ctrl = ui.input(|input| input.modifiers.command);
            if reading || ctrl {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            let clicked = if reading {
                response.clicked()
            } else {
                ctrl && ui.input(|input| input.pointer.primary_clicked())
            };
            if clicked && is_web(href) {
                frame.painted.link = Some(href.clone());
            }
        }
    }
    paint_leaf(ui, path, rect.min, galley, style.colour, frame);
}

/// The selection under the text, then the text; the leaf kept for the editor.
fn paint_leaf(
    ui: &Ui,
    path: &[usize],
    origin: Pos2,
    galley: Arc<Galley>,
    colour: Color32,
    frame: &mut Frame<'_>,
) {
    let p = palette(ui.ctx());
    for (_, range) in frame.found.iter().filter(|(at, _)| at.as_slice() == path) {
        for rect in crate::widgets::editor::selection_rects(&galley, range.clone(), false, 4.0) {
            ui.painter()
                .rect_filled(rect.translate(origin.to_vec2()), 2.0, p.bg_3);
        }
    }
    if let Some(selection) = frame.selection {
        let (start, end) = richtext::edit::ordered(selection);
        let path = path.to_vec();
        if start.path <= path && path <= end.path && start != end {
            let from = if path == start.path { start.offset } else { 0 };
            let to = if path == end.path {
                end.offset
            } else {
                galley.text().chars().count()
            };
            let past_end = path != end.path;
            let fill = p.amber_fill.gamma_multiply(0.3);
            for rect in crate::widgets::editor::selection_rects(&galley, from..to, past_end, 4.0) {
                ui.painter()
                    .rect_filled(rect.translate(origin.to_vec2()), 2.0, fill);
            }
        }
    }
    ui.painter().galley(origin, Arc::clone(&galley), colour);
    if frame.selection.is_some() {
        frame.painted.leaves.push(Laid {
            path: path.to_vec(),
            origin,
            galley,
        });
    }
}

/// `http` and `https` alone: a Note's link never opens a file or runs a scheme.
fn is_web(href: &str) -> bool {
    let lower = href.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn list(ui: &mut Ui, kind: ListKind, items: &[Item], path: &mut Vec<usize>, frame: &mut Frame<'_>) {
    let p = palette(ui.ctx());
    let size = frame.size;
    let indent = if kind == ListKind::Task {
        14.0 * size / BODY + 8.0
    } else {
        INDENT * size
    };
    for (at, item) in items.iter().enumerate() {
        if at > 0 {
            ui.add_space(0.2 * size);
        }
        path.push(at);
        ui.horizontal_top(|ui| {
            let line = size * LINE_HEIGHT;
            let (marker, _) = ui.allocate_exact_size(vec2(indent, line), Sense::hover());
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
                ListKind::Task => {
                    let rect = checkbox(ui, marker, item.checked, size);
                    let sense = if frame.selection.is_some() {
                        Sense::click()
                    } else {
                        Sense::hover()
                    };
                    let response =
                        crate::widgets::region(ui, rect.expand(3.0), path.clone(), sense);
                    let checked = item.checked;
                    let name = match item.blocks.first() {
                        Some(Block::Paragraph(content)) if !content.is_empty() => {
                            richtext::inlines::plain(content)
                        }
                        // Named still when empty: by its place in the list.
                        _ => (at + 1).to_string(),
                    };
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(egui::Role::CheckBox, true, checked, &name)
                    });
                    crate::widgets::address(ui, &response, "rich-task");
                    if response.clicked() {
                        frame.painted.ticked = Some((path[..path.len() - 1].to_vec(), at));
                    }
                }
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                for (index, block) in item.blocks.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(0.5 * size);
                    }
                    path.push(index);
                    match block {
                        Block::Paragraph(content) if kind == ListKind::Task => {
                            let style = Style {
                                struck: item.checked,
                                ..Style::body(p, size)
                            };
                            paragraph(ui, content, style, path, frame);
                        }
                        _ => draw_block(ui, block, path, frame),
                    }
                    path.pop();
                }
            });
        });
        path.pop();
    }
}

/// A task's box: amber when done, as the reference's checkbox.
fn checkbox(ui: &Ui, marker: Rect, checked: bool, size: f32) -> Rect {
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
    rect
}

fn quote(ui: &mut Ui, inner: &[Block], path: &mut Vec<usize>, frame: &mut Frame<'_>) {
    let p = palette(ui.ctx());
    let size = frame.size;
    let top = ui.cursor().top();
    ui.horizontal_top(|ui| {
        ui.add_space(14.0 + 3.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.add_space(2.0);
            for (at, block) in inner.iter().enumerate() {
                if at > 0 {
                    ui.add_space(GAP * size);
                }
                path.push(at);
                match block {
                    Block::Paragraph(content) => {
                        let style = Style {
                            colour: p.text_1,
                            ..Style::body(p, size)
                        };
                        paragraph(ui, content, style, path, frame);
                    }
                    _ => draw_block(ui, block, path, frame),
                }
                path.pop();
            }
            ui.add_space(2.0);
        });
    });
    let bottom = ui.cursor().top();
    let left = ui.max_rect().left();
    let bar = Rect::from_min_max(pos2(left, top), pos2(left + 3.0, bottom));
    ui.painter()
        .rect_filled(bar, CornerRadius::ZERO, p.amber_dim);
}

fn code_block(ui: &mut Ui, language: &str, text: &str, path: &[usize], frame: &mut Frame<'_>) {
    let is_diagram = language == diagram::MERMAID && !text.trim().is_empty();
    let shown = if is_diagram {
        diagram_of(ui, text, frame)
    } else {
        None
    };
    let folded = is_diagram
        && frame.diagrams.is_some()
        && !matches!(shown, Some(Diagram::Unsupported))
        && !caret_in(frame.selection, path);
    let p = palette(ui.ctx());
    let font = FontId::monospace(CODE_SIZE * frame.size / BODY);
    let syntax = syntax_of(language);
    let lines = syntax.map(|syntax| crate::services::highlight::short(ui.ctx(), syntax, text));
    let mut job = LayoutJob::default();
    for (at, line) in text.split('\n').enumerate() {
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
    let height = if folded {
        galley.rows.first().map_or(0.0, |row| row.rect().height())
    } else {
        galley.size().y
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, height + 2.0 * padding.y), Sense::hover());
    if frame.selection.is_none() {
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Code, true, text));
    }
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        p.bg_1,
        Stroke::new(1.0, p.line_soft),
        StrokeKind::Inside,
    );
    if !language.is_empty() {
        ui.painter().text(
            pos2(rect.right() - 10.0, rect.top() + 6.0),
            Align2::RIGHT_TOP,
            language,
            FontId::monospace(10.0),
            p.text_2,
        );
    }
    let mut clipped = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    clipped.set_clip_rect(rect.shrink2(vec2(0.0, padding.y - 2.0)));
    paint_leaf(&clipped, path, rect.min + padding, galley, p.text_0, frame);
    if is_diagram {
        diagram_under(ui, shown, path, frame);
    }
}

/// Its drawing: the one of its source, or else the last one drawn in its place, while the new
/// one is asked.
fn diagram_of(ui: &Ui, text: &str, frame: &mut Frame<'_>) -> Option<Diagram> {
    let (diagrams, _) = frame.diagrams?;
    let place = frame.places.with(frame.rank);
    frame.rank += 1;
    if let Some(drawn) = diagrams(text) {
        ui.data_mut(|data| data.insert_temp(place, drawn.clone()));
        return Some(drawn);
    }
    let last = ui.data(|data| data.get_temp::<Diagram>(place));
    frame.painted.sketches.push(Sketch {
        source: text.to_owned(),
        stale: last.is_some(),
    });
    last
}

fn caret_in(selection: Option<&Selection>, path: &[usize]) -> bool {
    selection.is_some_and(|selection| selection.anchor.path == path && selection.head.path == path)
}

/// The drawing, or why there is none; nothing before its first drawing.
fn diagram_under(ui: &mut Ui, shown: Option<Diagram>, path: &[usize], frame: &mut Frame<'_>) {
    let (Some((_, language)), Some(shown)) = (frame.diagrams, shown) else {
        return;
    };
    let p = palette(ui.ctx());
    let tr = |key, params: &[(&str, i18n::Value)]| i18n::translate(language, key, params);
    ui.add_space(6.0);
    match shown {
        Diagram::Drawn(texture) => {
            let padding = 12.0;
            let natural = texture.size_vec2() / diagram::SCALE;
            let room = ui.available_width() - 2.0 * padding;
            let size = natural * (room / natural.x).min(1.0);
            let (rect, _) = ui.allocate_exact_size(
                vec2(ui.available_width(), size.y + 2.0 * padding),
                Sense::hover(),
            );
            let response = crate::widgets::region(ui, rect, ("rich-diagram", path), Sense::click());
            let edit = tr(keys::editor::rich::diagram::EDIT, &[]);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, &edit));
            crate::widgets::address(ui, &response, "rich-diagram");
            let response = response.on_hover_text(edit);
            ui.painter().rect(
                rect,
                CornerRadius::same(8),
                p.bg_1,
                Stroke::new(1.0, p.line_soft),
                StrokeKind::Inside,
            );
            let picture = Rect::from_center_size(rect.center(), size);
            let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            ui.painter()
                .image(texture.id(), picture, whole, Color32::WHITE);
            if response.clicked() {
                frame.painted.diagram = Some(path.to_vec());
            }
        }
        Diagram::Failed { line, detail } => {
            let sentence = match line {
                Some(line) => tr(
                    keys::editor::rich::diagram::FAILED_AT,
                    &[("line", i18n::Value::from(line.to_string()))],
                ),
                None => tr(keys::editor::rich::diagram::FAILED, &[]),
            };
            said(
                ui,
                &sentence,
                p.red,
                FontFamily::Proportional,
                "rich-diagram-error",
            );
            ui.add_space(6.0);
            said(
                ui,
                &detail,
                p.text_1,
                FontFamily::Monospace,
                "rich-diagram-detail",
            );
        }
        Diagram::Unsupported => {
            let words = tr(keys::editor::rich::diagram::UNSUPPORTED, &[]);
            said(
                ui,
                &words,
                p.text_2,
                FontFamily::Proportional,
                "rich-diagram-unsupported",
            );
        }
    }
}

/// A line of words under a diagram's code, wrapped to the text's width.
fn said(ui: &mut Ui, words: &str, colour: Color32, family: FontFamily, author_id: &str) {
    let font = FontId::new(12.0, family);
    let galley = ui
        .painter()
        .layout(words.to_owned(), font, colour, ui.available_width());
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, words));
    crate::widgets::address(ui, &response, author_id);
    ui.painter().galley(rect.min, galley, colour);
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
    let plain = |colour| TextFormat::simple(font.clone(), colour);
    let Some(tokens) = tokens else {
        job.append(line, 0.0, plain(p.text_0));
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
            job.append(&line[from..start], 0.0, plain(p.text_0));
        }
        let (colour, leans) = crate::services::highlight::colour(p, *token);
        let format = TextFormat {
            italics: leans,
            ..plain(colour)
        };
        job.append(&line[start..end.max(start)], 0.0, format);
        from = end.max(start);
    }
    if from < line.len() {
        job.append(&line[from..], 0.0, plain(p.text_0));
    }
}

fn table(ui: &mut Ui, rows: &[Vec<richtext::Cell>], path: &[usize], frame: &mut Frame<'_>) {
    let p = palette(ui.ctx());
    let size = frame.size;
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    if columns == 0 {
        return;
    }
    let width = ui.available_width();
    let column = width / f32::from(u16::try_from(columns).unwrap_or(u16::MAX));
    let padding = vec2(10.0, 6.0);
    for (at, row) in rows.iter().enumerate() {
        let header = at == 0;
        let style = Style {
            weight: if header { 600.0 } else { 400.0 },
            ..Style::body(p, size)
        };
        let galleys: Vec<_> = (0..columns)
            .map(|index| {
                let cell = row.get(index).map_or(&[][..], Vec::as_slice);
                let (job, _) = job(p, cell, style, column - 2.0 * padding.x);
                ui.fonts_mut(|fonts| fonts.layout_job(job))
            })
            .collect();
        let height = galleys
            .iter()
            .map(|galley| galley.size().y)
            .fold(size * LINE_HEIGHT, f32::max)
            + 2.0 * padding.y;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        for (index, galley) in galleys.into_iter().enumerate() {
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
            let cell_path = [path, &[at, index]].concat();
            paint_leaf(ui, &cell_path, cell.min + padding, galley, p.text_0, frame);
        }
    }
}

/// A drawing embedded by `devnotes:drawing/<id>`, its picture read by id where it is shown; any
/// other image as it was written: a Note never fetches an address.
/// The drawing at its own size, never wider than the text.
fn picture_of(ui: &mut Ui, texture: &egui::TextureHandle, alt: &str) {
    let natural = texture.size_vec2();
    let scale = (ui.available_width() / natural.x).min(1.0);
    let (rect, response) = ui.allocate_exact_size(natural * scale, Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, alt));
    crate::widgets::address(ui, &response, "rich-embed");
    ui.painter().image(
        texture.id(),
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

/// Where a picture is coming, or says it will not.
fn placeholder(ui: &mut Ui, words: &str, size: f32) {
    let p = palette(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 3.0 * size), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, words));
    crate::widgets::address(ui, &response, "rich-embed");
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

fn image(ui: &mut Ui, alt: &str, src: &str, frame: &mut Frame<'_>) {
    let size = frame.size;
    let id = src.strip_prefix(DRAWING_SOURCE);
    let embedded = id
        .zip(frame.pictures)
        .map(|(id, (pictures, missing))| (id, pictures(id), missing));
    if let Some((id, picture, missing)) = embedded {
        match picture {
            Some(Embedded::Shown(texture)) => return picture_of(ui, &texture, alt),
            Some(Embedded::Missing) => return placeholder(ui, missing, size),
            Some(Embedded::Waiting) => {}
            None => frame.painted.wanted.push(id.to_owned()),
        }
        return placeholder(ui, "", size);
    }
    let written = format!("![{alt}]({src})");
    let words = if id.is_some() { alt } else { written.as_str() };
    placeholder(ui, words, size);
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

    fn says<T>(harness: &egui_kittest::Harness<'_, T>, words: &str) -> bool {
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

    const FLOW: &str = "```mermaid\ngraph TD\n  A -->\n  B\n```";

    /// What the diagrams of a frame asked and answered.
    #[derive(Default)]
    struct Asked {
        answer: Option<Diagram>,
        sketches: Vec<(String, bool)>,
        clicked: Option<Vec<usize>>,
    }

    fn diagrams(markdown: &str, answer: Option<Diagram>) -> egui_kittest::Harness<'static, Asked> {
        let mut document = richtext::read(markdown);
        let mut harness = crate::testing::harness()
            .with_size(vec2(700.0, 900.0))
            .build_ui_state(
                move |ui, asked: &mut Asked| {
                    if let Some(Block::CodeBlock { text, .. }) = document.blocks.first_mut()
                        && asked.answer.is_none()
                        && !asked.sketches.is_empty()
                    {
                        text.push_str("\n  C");
                    }
                    let answer = asked.answer.clone();
                    let lookup = move |_: &str| answer.clone();
                    let painted = RichView::new(&document)
                        .diagrams(&lookup, i18n::Language::En)
                        .show(ui);
                    asked.sketches.extend(
                        painted
                            .sketches
                            .into_iter()
                            .map(|sketch| (sketch.source, sketch.stale)),
                    );
                    asked.clicked = asked.clicked.take().or(painted.diagram);
                },
                Asked {
                    answer,
                    ..Asked::default()
                },
            );
        harness.run();
        harness
    }

    fn code_height(harness: &egui_kittest::Harness<'_, Asked>) -> f32 {
        harness
            .get_by(|node| {
                node.label()
                    .is_some_and(|code| code.starts_with("graph TD"))
            })
            .rect()
            .height()
    }

    #[test]
    fn a_diagram_that_cannot_be_read_names_its_line_under_its_folded_code() {
        let failed = Diagram::Failed {
            line: Some(2),
            detail: "unexpected end of input".to_owned(),
        };
        let harness = diagrams(FLOW, Some(failed));

        assert!(says(&harness, "The diagram cannot be read, line 2."));
        assert!(says(&harness, "unexpected end of input"));
        let unsupported = diagrams(FLOW, Some(Diagram::Unsupported));
        assert!(
            code_height(&harness) + 30.0 < code_height(&unsupported),
            "folded to its first line"
        );
    }

    #[test]
    fn a_type_not_supported_stays_code_and_says_so() {
        let harness = diagrams(FLOW, Some(Diagram::Unsupported));

        assert!(says(&harness, "Diagram type not supported"));
        assert!(harness.state().sketches.is_empty(), "nothing more to draw");
    }

    #[test]
    fn a_diagram_not_drawn_is_asked_then_its_last_drawing_stands_in_while_retyped() {
        let mut harness = diagrams(FLOW, None);
        assert_eq!(
            harness.state().sketches.first(),
            Some(&("graph TD\n  A -->\n  B".to_owned(), false)),
            "asked with nothing in its place"
        );

        harness.state_mut().answer = Some(Diagram::Unsupported);
        harness.run();
        harness.state_mut().answer = None;
        harness.state_mut().sketches.clear();
        harness.run();

        let stale = harness
            .state()
            .sketches
            .iter()
            .find(|(source, _)| source.ends_with('C'));
        assert_eq!(stale.map(|(_, stale)| *stale), Some(true));
        assert!(
            says(&harness, "Diagram type not supported"),
            "the last answer stays"
        );
    }

    #[test]
    fn a_drawing_clicked_goes_to_its_code() {
        let mut harness = crate::testing::harness()
            .with_size(vec2(700.0, 900.0))
            .build_ui_state(
                |ui, clicked: &mut Option<Vec<usize>>| {
                    let image = egui::ColorImage::filled([200, 100], Color32::RED);
                    let texture =
                        ui.ctx()
                            .load_texture("drawn", image, egui::TextureOptions::default());
                    let lookup = move |_: &str| Some(Diagram::Drawn(texture.clone()));
                    let document = richtext::read(FLOW);
                    let selection = Selection::default();
                    let painted = RichView::new(&document)
                        .editing(&selection)
                        .diagrams(&lookup, i18n::Language::En)
                        .show(ui);
                    *clicked = clicked.take().or(painted.diagram);
                },
                None,
            );
        harness.run();

        harness
            .get_by_label("Click to edit the diagram’s code")
            .click();
        harness.run();

        assert_eq!(harness.state().as_deref(), Some(&[0][..]));
    }
}
