//! A run of the documentation's text — its marks, its key caps, its links — laid out as one
//! galley, so that a sentence wraps as one.

use std::ops::Range;
use std::sync::Arc;

use egui::text::{CCursor, LayoutJob};
use egui::{
    Align, Color32, CornerRadius, FontFamily, FontId, Galley, Pos2, Rect, Role, Sense, Stroke,
    StrokeKind, TextFormat, Ui, Vec2, WidgetInfo, vec2,
};
use engine::docs::model::{Inline, Target, text_of};

use crate::services::shortcuts::{self, Bindings};
use crate::theme::icons::{self, Icon};
use crate::theme::tokens::{Palette, font, gap, radius};
use crate::theme::{focus, palette};
use crate::widgets;

const LINE_HEIGHT: f32 = 1.6;
/// A key's cap over its row, above and under its letters.
const CAP_HEIGHT: f32 = 17.0;

/// How a text is written before its marks.
#[derive(Clone, Copy)]
pub(super) struct Style {
    pub size: f32,
    pub weight: f32,
    pub colour: Color32,
}

impl Style {
    pub(super) fn body(p: &Palette) -> Self {
        Self {
            size: font::READING,
            weight: 400.0,
            colour: p.text_1,
        }
    }
}

/// A text laid out, with what is painted under it and answers over it.
pub(super) struct Laid<'a> {
    galley: Arc<Galley>,
    style: Style,
    spoken: String,
    caps: Vec<Range<usize>>,
    /// Painted as icons over the room kept for them: none of the embedded faces has an arrow.
    arrows: Vec<(Range<usize>, Icon, Color32)>,
    links: Vec<(Range<usize>, &'a str, &'a Target)>,
}

pub(super) fn lay<'a>(
    ui: &Ui,
    runs: &'a [Inline],
    style: Style,
    width: f32,
    bindings: &Bindings,
) -> Laid<'a> {
    let mut writer = Writer::new(palette(ui.ctx()), style);
    for run in runs {
        writer.run(run, bindings);
    }
    writer.job.wrap.max_width = width;
    let Writer {
        job,
        caps,
        arrows,
        links,
        ..
    } = writer;
    Laid {
        galley: ui.fonts_mut(|fonts| fonts.layout_job(job)),
        style,
        spoken: text_of(runs),
        caps,
        arrows,
        links,
    }
}

/// A paragraph over the room left: the link followed, if one was.
pub(super) fn paragraph<'a>(
    ui: &mut Ui,
    runs: &'a [Inline],
    style: Style,
    bindings: &Bindings,
) -> Option<&'a Target> {
    let width = ui.available_width();
    let laid = lay(ui, runs, style, width, bindings);
    let (rect, _) = ui.allocate_exact_size(vec2(width, laid.size().y), Sense::hover());
    laid.show(ui, rect.min)
}

impl<'a> Laid<'a> {
    pub(super) fn size(&self) -> Vec2 {
        self.galley.size()
    }

    /// Painted at `at`: the link followed, if one was.
    pub(super) fn show(self, ui: &mut Ui, at: Pos2) -> Option<&'a Target> {
        let p = palette(ui.ctx());
        let place = Rect::from_min_size(at, self.size());
        let node = widgets::region(ui, place, ("docs-text", at.y.to_bits()), Sense::hover());
        node.widget_info(|| WidgetInfo::labeled(Role::Label, true, &self.spoken));

        for cap in &self.caps {
            for rect in self.rects(cap, at) {
                let cap = Rect::from_center_size(rect.center(), vec2(rect.width(), CAP_HEIGHT));
                ui.painter().rect(
                    cap,
                    CornerRadius::same(radius::TAG),
                    p.bg_3,
                    Stroke::new(gap::HAIRLINE, p.line),
                    StrokeKind::Inside,
                );
            }
        }
        ui.painter()
            .galley(at, Arc::clone(&self.galley), self.style.colour);
        for (range, icon, ink) in &self.arrows {
            for rect in self.rects(range, at) {
                let side = rect.width().min(font::READING);
                let glyph = Rect::from_center_size(rect.center(), Vec2::splat(side));
                icons::paint(ui, glyph, *icon, *ink);
            }
        }
        self.follow(ui, at)
    }

    /// Each link is a control of its own, reached by Tab as by the pointer.
    fn follow(&self, ui: &mut Ui, at: Pos2) -> Option<&'a Target> {
        let mut followed = None;
        for (range, text, target) in &self.links {
            let rects = self.rects(range, at);
            let Some(first) = rects.first() else {
                continue;
            };
            for (row, rect) in rects.iter().enumerate() {
                let salt = ("docs-link", range.start, row, at.y.to_bits());
                // One stop of Tab per link, over the first row it takes.
                let sense = if rect == first {
                    Sense::click()
                } else {
                    Sense::CLICK
                };
                let link = widgets::region(ui, *rect, salt, sense);
                if rect == first {
                    link.widget_info(|| WidgetInfo::labeled(Role::Link, true, *text));
                    widgets::address(ui, &link, "docs-link");
                    focus::ring(ui, &link, CornerRadius::same(radius::TAG));
                }
                widgets::pointer(ui, &link);
                // Under the letters: egui draws it under the row, which is taller.
                let under = rect.center().y + self.style.size * 0.6;
                let stroke = Stroke::new(gap::HAIRLINE, palette(ui.ctx()).amber_text);
                ui.painter().hline(rect.x_range(), under, stroke);
                if link.clicked() {
                    followed = Some(*target);
                }
            }
        }
        followed
    }

    /// Where a range of characters is, a rectangle per row it takes.
    fn rects(&self, range: &Range<usize>, at: Pos2) -> Vec<Rect> {
        let mut rects = Vec::new();
        let mut start = 0;
        for row in &self.galley.rows {
            let end = start + row.char_count_including_newline().0;
            let (from, to) = (range.start.max(start), range.end.min(end));
            if from < to {
                // A cursor between two rows is read at the end of the first one.
                let x = |at: usize| self.galley.pos_from_cursor(CCursor::new(at)).left();
                let left = if from == start {
                    row.rect().left()
                } else {
                    x(from)
                };
                let right = if to == end { row.rect().right() } else { x(to) };
                let rect = Rect::from_x_y_ranges(left..=right, row.min_y()..=row.max_y());
                rects.push(rect.translate(at.to_vec2()));
            }
            start = end;
        }
        rects
    }
}

struct Writer<'a> {
    job: LayoutJob,
    chars: usize,
    palette: &'static Palette,
    style: Style,
    caps: Vec<Range<usize>>,
    arrows: Vec<(Range<usize>, Icon, Color32)>,
    links: Vec<(Range<usize>, &'a str, &'a Target)>,
}

impl<'a> Writer<'a> {
    fn new(palette: &'static Palette, style: Style) -> Self {
        Self {
            job: LayoutJob::default(),
            chars: 0,
            palette,
            style,
            caps: Vec::new(),
            arrows: Vec::new(),
            links: Vec::new(),
        }
    }

    fn run(&mut self, run: &'a Inline, bindings: &Bindings) {
        let p = self.palette;
        let plain = self.format(self.style.weight, self.style.colour);
        match run {
            Inline::Text(text) => self.write(text, &plain),
            Inline::Strong(text) | Inline::Label(text) => {
                let strong = self.format(650.0, p.text_0);
                self.write(text, &strong);
            }
            Inline::Code(text) => {
                let code = TextFormat {
                    font_id: FontId::new(self.style.size * 0.92, FontFamily::Monospace),
                    color: p.amber_text,
                    ..plain
                };
                self.write(text, &code);
            }
            Inline::Link { text, to } => {
                let start = self.chars;
                let link = self.format(self.style.weight, p.amber_text);
                self.write(text, &link);
                self.links.push((start..self.chars, text, to));
            }
            Inline::Keys(keys) => self.keys(&held(keys, bindings)),
        }
    }

    /// One cap per key; the spaces inside it never break, and make its sides.
    fn keys(&mut self, keys: &[String]) {
        let cap = TextFormat {
            font_id: FontId::new(font::META, FontFamily::Monospace),
            color: self.palette.text_0,
            valign: Align::Center,
            ..self.format(self.style.weight, self.palette.text_0)
        };
        let between = TextFormat {
            font_id: FontId::proportional(font::BADGE),
            ..cap.clone()
        };
        for (at, key) in keys.iter().enumerate() {
            if at > 0 {
                self.write("\u{a0}", &between);
            }
            let start = self.chars;
            self.write(&format!("\u{a0}{key}\u{a0}"), &cap);
            self.caps.push(start..self.chars);
        }
    }

    fn format(&self, weight: f32, colour: Color32) -> TextFormat {
        let mut format = TextFormat {
            font_id: FontId::proportional(self.style.size),
            color: colour,
            line_height: Some(self.style.size * LINE_HEIGHT),
            ..TextFormat::default()
        };
        format.coords.push(*b"wght", weight);
        format
    }

    fn write(&mut self, text: &str, format: &TextFormat) {
        // A monospace space is twice a proportional one: an arrow takes as much room in both.
        let room = match format.font_id.family {
            FontFamily::Monospace => 2,
            _ => 4,
        };
        let mut laid = String::with_capacity(text.len());
        for glyph in text.chars() {
            let start = self.chars;
            if let Some(icon) = widgets::arrow_of(glyph) {
                laid.extend(std::iter::repeat_n('\u{a0}', room));
                self.chars += room;
                self.arrows.push((start..self.chars, icon, format.color));
            } else {
                laid.push(glyph);
                self.chars += 1;
            }
        }
        self.job.append(&laid, 0.0, format.clone());
    }
}

/// The keys really bound, where the page names the combination that shipped: a letter alone is
/// left as it is written, it means something else elsewhere.
pub(super) fn held(keys: &[String], bindings: &Bindings) -> Vec<String> {
    let shipped = keys.join("+");
    shortcuts::every()
        .find(|action| keys.len() > 1 && action.fallback == shipped)
        .map_or_else(
            || keys.to_vec(),
            |action| bindings.of(action).split('+').map(str::to_owned).collect(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_combination_moved_in_the_preferences_is_the_one_shown() {
        let keys = |text: &str| -> Vec<String> { text.split(' ').map(str::to_owned).collect() };
        let mut bindings = Bindings::default();
        assert_eq!(
            held(&keys("Ctrl Alt P"), &bindings),
            keys("Ctrl Alt P"),
            "nothing moved"
        );

        let palette = shortcuts::every()
            .find(|action| action.id == shortcuts::PALETTE)
            .expect("the palette's");
        bindings.set(palette, "Ctrl+Alt+J".to_owned());
        assert_eq!(held(&keys("Ctrl Alt P"), &bindings), keys("Ctrl Alt J"));
        assert_eq!(held(&keys("Ctrl F"), &bindings), keys("Ctrl F"));
        assert_eq!(held(&keys("C"), &bindings), keys("C"));
    }
}
