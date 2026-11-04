//! Where each line sits. Only the lines on screen are laid out; the others count as many rows as
//! their length would wrap to, until they are shown and measured. Unwrapped, every line is one
//! row, and nothing is estimated.

use std::sync::Arc;

use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{Color32, FontId, Galley, Pos2, Rect, Ui, Vec2};
use text::Buffer;

use super::{Colours, Decoration, Mark};
use crate::theme::palette;

/// How much further in than the line's own text a wrapped row starts.
const HANG_COLUMNS: usize = 2;

/// What a frame lays out with.
pub(crate) struct Frame {
    pub font: FontId,
    pub row_height: f32,
    pub char_width: f32,
    /// The text's width when lines wrap.
    pub wrap: Option<f32>,
    /// Where a line's text starts, on screen.
    pub text_left: f32,
    pub top: f32,
}

/// The lines' heights, known or estimated, and where each one starts.
#[derive(Debug, Default)]
pub struct Lines {
    heights: Vec<Option<f32>>,
    /// `tops[i]` is line `i`'s, from the text's top; one more than the lines, the total last.
    tops: Vec<f32>,
    /// The longest line's width unwrapped, kept until an edit may have changed it.
    widest: Option<f32>,
    wrap: Option<f32>,
}

impl Lines {
    /// From this line on, the heights are measured again.
    pub(crate) fn forget_from(&mut self, line: usize) {
        self.heights.truncate(line);
        self.widest = None;
    }

    pub(crate) fn measure(&mut self, buffer: &Buffer, frame: &Frame) {
        if self.wrap != frame.wrap {
            self.heights.clear();
            self.wrap = frame.wrap;
        }
        let count = buffer.line_count();
        self.heights.resize(count, None);
        self.tops.clear();
        self.tops.reserve(count + 1);
        let mut top = 0.0;
        for (line, height) in self.heights.iter().enumerate() {
            self.tops.push(top);
            top += height.unwrap_or_else(|| estimate(buffer, line, frame));
        }
        self.tops.push(top);
    }

    pub(crate) fn total(&self) -> f32 {
        self.tops.last().copied().unwrap_or(0.0)
    }

    pub(crate) fn top(&self, line: usize) -> f32 {
        self.tops.get(line).copied().unwrap_or(0.0)
    }

    /// The line at `y` from the text's top, the last one below the text.
    pub(crate) fn line_at(&self, y: f32) -> usize {
        let lines = self.tops.len().saturating_sub(1);
        let after = self.tops.partition_point(|top| *top <= y);
        after.saturating_sub(1).min(lines.saturating_sub(1))
    }

    /// What a line measured once laid out: its rows from now on.
    pub(crate) fn set_height(&mut self, line: usize, height: f32) {
        if let Some(known) = self.heights.get_mut(line) {
            *known = Some(height);
        }
    }

    /// Unwrapped, as wide as the longest line; wrapped, the text's width.
    pub(crate) fn widest(&mut self, buffer: &Buffer, frame: &Frame) -> f32 {
        if let Some(width) = frame.wrap {
            return width;
        }
        *self.widest.get_or_insert_with(|| {
            let longest = (0..buffer.line_count())
                .map(|line| buffer.rope().line(line).len_chars())
                .max()
                .unwrap_or(0);
            #[expect(clippy::cast_precision_loss, reason = "a line's length")]
            let width = (longest + 2) as f32 * frame.char_width;
            width
        })
    }
}

/// A monospace line's rows if it wraps at its width; one otherwise.
fn estimate(buffer: &Buffer, line: usize, frame: &Frame) -> f32 {
    let Some(width) = frame.wrap else {
        return frame.row_height;
    };
    #[expect(clippy::cast_precision_loss, reason = "a line's length")]
    let wide = buffer.rope().line(line).len_chars() as f32 * frame.char_width;
    (wide / width).ceil().max(1.0) * frame.row_height
}

/// What one line is drawn with besides its text.
pub(crate) struct Paint<'a> {
    pub colours: Option<Colours<'a>>,
    pub decorations: &'a [Decoration],
}

/// Line `line` laid out at its place's width, its wrapped rows hung under its indent. Coloured
/// and decorated when `paint` is given; plain for hit-testing, which only reads positions.
pub(crate) fn galley(
    ui: &Ui,
    buffer: &Buffer,
    line: usize,
    frame: &Frame,
    paint: Option<&Paint<'_>>,
) -> Arc<Galley> {
    let text = buffer.line(line);
    let p = palette(ui.ctx());
    let hang = frame.wrap.map(|width| {
        let leading = text
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .map(|c| if c == '\t' { 4 } else { 1 })
            .sum::<usize>();
        #[expect(clippy::cast_precision_loss, reason = "a few columns")]
        let hang = (leading + HANG_COLUMNS) as f32 * frame.char_width;
        hang.min(width / 2.0)
    });
    let mut job = LayoutJob::default();
    job.wrap.max_width = frame
        .wrap
        .map_or(f32::INFINITY, |width| width - hang.unwrap_or(0.0));
    let start = buffer.line_start(line);
    let runs = paint.map_or_else(Vec::new, |paint| runs(&text, line, start, paint, p));
    let mut at = 0;
    let mut byte = 0;
    let mut push = |job: &mut LayoutJob, to: usize, colour: Color32, background: Color32| {
        let end = text
            .char_indices()
            .nth(to)
            .map_or(text.len(), |(index, _)| index);
        let format = TextFormat {
            font_id: frame.font.clone(),
            color: colour,
            background,
            ..TextFormat::default()
        };
        job.append(&text[byte..end], 0.0, format);
        byte = end;
    };
    for (to, colour, background) in runs {
        if to > at {
            push(&mut job, to, colour, background);
            at = to;
        }
    }
    push(
        &mut job,
        text.chars().count(),
        p.text_0,
        Color32::TRANSPARENT,
    );
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    match hang {
        Some(hang) if galley.rows.len() > 1 => {
            let mut hanging = (*galley).clone();
            for row in hanging.rows.iter_mut().skip(1) {
                row.pos.x += hang;
            }
            hanging.rect.max.x += hang;
            Arc::new(hanging)
        }
        _ => galley,
    }
}

/// The line cut where its colour or its decoration changes: each run's end and look.
fn runs(
    text: &str,
    line: usize,
    start: usize,
    paint: &Paint<'_>,
    p: &crate::theme::tokens::Palette,
) -> Vec<(usize, Color32, Color32)> {
    let length = text.chars().count();
    let colours = paint
        .colours
        .map_or_else(Vec::new, |colours| colours(line, text));
    let decorations: Vec<(std::ops::Range<usize>, Mark)> = paint
        .decorations
        .iter()
        .filter(|decoration| {
            decoration.range.start < start + length && decoration.range.end > start
        })
        .map(|decoration| {
            let from = decoration.range.start.saturating_sub(start);
            let to = (decoration.range.end - start).min(length);
            (from..to, decoration.mark)
        })
        .collect();
    let mut cuts: Vec<usize> = colours
        .iter()
        .flat_map(|(range, _)| [range.start, range.end])
        .chain(
            decorations
                .iter()
                .flat_map(|(range, _)| [range.start, range.end]),
        )
        .filter(|cut| *cut <= length)
        .collect();
    cuts.push(length);
    cuts.sort_unstable();
    cuts.dedup();
    let mut runs = Vec::with_capacity(cuts.len());
    let mut from = 0;
    for to in cuts {
        if to == from {
            continue;
        }
        let inside = |range: &std::ops::Range<usize>| range.start <= from && from < range.end;
        let mark = decorations
            .iter()
            .filter(|(range, _)| inside(range))
            .map(|(_, mark)| *mark)
            .max_by_key(|mark| *mark as u8);
        let colour = colours
            .iter()
            .rev()
            .find(|(range, _)| inside(range))
            .map_or(p.text_0, |(_, colour)| *colour);
        let (colour, background) = match mark {
            Some(Mark::Field) => (p.amber_text, p.amber_fill.gamma_multiply(0.35)),
            Some(Mark::Match) => (colour, p.bg_3),
            Some(Mark::CurrentMatch) => (colour, p.amber_dim),
            None => (colour, Color32::TRANSPARENT),
        };
        runs.push((to, colour, background));
        from = to;
    }
    runs
}

/// Where `line` is drawn: the text's left, the line's top.
pub(crate) fn origin(lines: &Lines, line: usize, frame: &Frame) -> Pos2 {
    Pos2::new(frame.text_left, frame.top + lines.top(line))
}

/// The caret's rectangle at `at`, on screen.
pub(crate) fn caret_rect(
    ui: &Ui,
    buffer: &Buffer,
    lines: &Lines,
    at: usize,
    frame: &Frame,
) -> Rect {
    let line = buffer.line_of(at);
    let column = at - buffer.line_start(line);
    let galley = galley(ui, buffer, line, frame, None);
    let local = galley.pos_from_cursor(CCursor::new(column));
    local.translate(origin(lines, line, frame).to_vec2())
}

/// The position under `pos`, on screen.
pub(crate) fn hit(ui: &Ui, buffer: &Buffer, lines: &Lines, pos: Pos2, frame: &Frame) -> usize {
    if pos.y < frame.top {
        return 0;
    }
    let line = lines.line_at(pos.y - frame.top);
    let galley = galley(ui, buffer, line, frame, None);
    let local = pos - origin(lines, line, frame);
    let column = galley.cursor_from_pos(Vec2::new(local.x, local.y)).index.0;
    buffer.line_start(line) + column.min(buffer.line(line).chars().count())
}
