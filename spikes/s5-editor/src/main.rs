//! Spike S5: the text core (`ropey`) and a minimal `EditorView` on 1 MB of TypeScript.
//!
//! Each simulated keystroke: insert into the rope, record undo, re-tokenise from the nearest
//! syntect checkpoint up to the end of the viewport, lay out the visible lines (hanging indent),
//! paint caret, selection and a completion bubble, tessellate. Reports p50/p95/p99.

use std::time::Instant;

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Pos2, Rect, pos2, vec2};
use ropey::Rope;
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

const CHECKPOINT_EVERY: usize = 128;
const LINE_H: f32 = 18.0;

struct Buffer {
    rope: Rope,
    undo: Vec<(usize, usize, String)>,
    /// Parser state at the start of every `CHECKPOINT_EVERY`-th line; valid up to `valid`.
    checkpoints: Vec<(ParseState, ScopeStack)>,
    valid: usize,
}

impl Buffer {
    fn insert(&mut self, char_at: usize, text: &str) {
        self.rope.insert(char_at, text);
        self.undo.push((char_at, text.chars().count(), String::new()));
        let line = self.rope.char_to_line(char_at);
        self.valid = self.valid.min(line / CHECKPOINT_EVERY);
    }

    fn undo(&mut self) {
        if let Some((at, len, _)) = self.undo.pop() {
            self.rope.remove(at..at + len);
            let line = self.rope.char_to_line(at);
            self.valid = self.valid.min(line / CHECKPOINT_EVERY);
        }
    }

    /// Scopes per visible line: resumes from the last valid checkpoint, extends checkpoints
    /// up to `last`. Returns one Vec<(byte range, colour)> per line in `first..=last`.
    fn highlight(&mut self, set: &SyntaxSet, syntax: &SyntaxReference, first: usize, last: usize) -> Vec<Vec<(usize, usize, Color32)>> {
        let start_cp = self.valid.min(first / CHECKPOINT_EVERY);
        self.checkpoints.truncate(start_cp + 1);
        if self.checkpoints.is_empty() {
            self.checkpoints.push((ParseState::new(syntax), ScopeStack::new()));
        }
        let (mut state, mut stack) = self.checkpoints[start_cp].clone();
        let mut out = Vec::new();
        let mut line_text = String::new();
        for line in start_cp * CHECKPOINT_EVERY..=last.min(self.rope.len_lines() - 1) {
            if line % CHECKPOINT_EVERY == 0 && line / CHECKPOINT_EVERY >= self.checkpoints.len() {
                self.checkpoints.push((state.clone(), stack.clone()));
            }
            line_text.clear();
            line_text.extend(self.rope.line(line).chunks());
            let ops = state.parse_line(&line_text, set).expect("parsed");
            if line >= first {
                let mut spans = Vec::new();
                let mut at = 0;
                for (pos, op) in &ops {
                    spans.push((at, *pos, colour_of(&stack)));
                    stack.apply(op).expect("scope");
                    at = *pos;
                }
                spans.push((at, line_text.len(), colour_of(&stack)));
                out.push(spans);
            } else {
                for (_, op) in &ops {
                    stack.apply(op).expect("scope");
                }
            }
        }
        self.valid = self.checkpoints.len() - 1;
        out
    }
}

/// The seven `tok-*` classes, by TextMate scope prefix.
fn colour_of(stack: &ScopeStack) -> Color32 {
    let Some(scope) = stack.as_slice().last() else { return Color32::from_gray(210) };
    let name = scope.build_string();
    match () {
        _ if name.starts_with("keyword") || name.starts_with("storage") => Color32::from_rgb(198, 120, 221),
        _ if name.starts_with("string") => Color32::from_rgb(152, 195, 121),
        _ if name.starts_with("constant") => Color32::from_rgb(209, 154, 102),
        _ if name.starts_with("entity.name") => Color32::from_rgb(97, 175, 239),
        _ if name.starts_with("comment") => Color32::from_gray(120),
        _ => Color32::from_gray(210),
    }
}

/// One wrapped line, continuation rows shifted right by the line's own indent plus one unit:
/// `TextEdit` cannot do this, a galley's rows can be moved.
fn hanging(ctx: &egui::Context, text: &str, spans: &[(usize, usize, Color32)], width: f32) -> std::sync::Arc<egui::Galley> {
    let indent_chars = text.chars().take_while(|c| *c == ' ').count() + 2;
    let font = FontId::monospace(13.0);
    let mut job = LayoutJob::default();
    for (from, to, colour) in spans {
        let piece = text.get(*from..*to).unwrap_or("").trim_end_matches('\n');
        job.append(piece, 0.0, TextFormat::simple(font.clone(), *colour));
    }
    let char_w = ctx.fonts_mut(|f| f.glyph_width(&font, 'm'));
    let indent = indent_chars as f32 * char_w;
    job.wrap.max_width = (width - indent).max(char_w * 20.0);
    let galley = ctx.fonts_mut(|f| f.layout_job(job));
    if galley.rows.len() == 1 {
        return galley;
    }
    let mut shifted = (*galley).clone();
    for row in shifted.rows.iter_mut().skip(1) {
        row.pos.x += indent;
    }
    shifted.rect = shifted.rect.union(Rect::from_min_size(Pos2::ZERO, vec2(width, shifted.rect.height())));
    std::sync::Arc::new(shifted)
}

fn percentile(samples: &mut [f64], p: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[((samples.len() as f64 - 1.0) * p).round() as usize]
}

fn main() {
    let text = std::fs::read_to_string("out/inputs/1mb.ts").expect("run from spikes/ after s5-highlight");
    let set = two_face::syntax::extra_newlines();
    let syntax = set.find_syntax_by_extension("ts").expect("ts").clone();
    let started = Instant::now();
    let mut buffer = Buffer { rope: Rope::from_str(&text), undo: Vec::new(), checkpoints: Vec::new(), valid: 0 };
    println!("rope from 1 MB: {:.1} ms, {} lines", started.elapsed().as_secs_f64() * 1e3, buffer.rope.len_lines());

    let ctx = egui::Context::default();
    let size = vec2(1100.0, 900.0);
    let visible = (size.y / LINE_H) as usize + 1;
    let mut time = 0.0;
    let mut frame = |buffer: &mut Buffer, first_line: usize, caret: usize, events: Vec<egui::Event>| -> f64 {
        time += 1.0 / 60.0;
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), time: Some(time), events, ..Default::default() };
        let started = Instant::now();
        let output = ctx.run_ui(input, |ui| {
            let last = first_line + visible;
            let spans = buffer.highlight(&set, &syntax, first_line, last);
            let painter = ui.painter();
            let gutter = 56.0;
            let mut y = 0.0;
            let mut line_text = String::new();
            for (n, line_spans) in spans.iter().enumerate() {
                let number = first_line + n;
                line_text.clear();
                line_text.extend(buffer.rope.line(number).chunks());
                painter.text(pos2(gutter - 8.0, y), egui::Align2::RIGHT_TOP, (number + 1).to_string(), FontId::monospace(12.0), Color32::from_gray(90));
                let galley = hanging(ui.ctx(), &line_text, line_spans, size.x - gutter - 16.0);
                let height = galley.rect.height().max(LINE_H);
                if number == buffer.rope.char_to_line(caret) {
                    let column = caret - buffer.rope.line_to_char(number);
                    let at = galley.pos_from_cursor(egui::text::CCursor::new(column));
                    painter.rect_filled(Rect::from_min_size(pos2(gutter, y), vec2(size.x, height)), 0.0, Color32::from_gray(36));
                    painter.vline(gutter + at.min.x, (y + at.min.y)..=(y + at.max.y), egui::Stroke::new(2.0, Color32::from_rgb(232, 168, 56)));
                    // Completion bubble under the caret, as Calcul needs.
                    let bubble = Rect::from_min_size(pos2(gutter + at.min.x, y + height + 2.0), vec2(220.0, 5.0 * 20.0));
                    painter.rect_filled(bubble, 6.0, Color32::from_gray(44));
                    for (i, word) in ["round", "rounddown", "roundup", "root", "row"].iter().enumerate() {
                        painter.text(bubble.min + vec2(8.0, 4.0 + i as f32 * 20.0), egui::Align2::LEFT_TOP, *word, FontId::monospace(13.0), Color32::from_gray(220));
                    }
                }
                painter.galley(pos2(gutter, y), galley, Color32::WHITE);
                y += height;
                if y > size.y {
                    break;
                }
            }
        });
        ctx.tessellate(output.shapes, output.pixels_per_point);
        started.elapsed().as_secs_f64() * 1e3
    };

    // Open at the top, then jump deep into the file (checkpoints must be built up to there).
    println!("first frame at the top: {:.1} ms", frame(&mut buffer, 0, 0, vec![]));
    let deep = buffer.rope.len_lines() / 2;
    let at = buffer.rope.line_to_char(deep);
    println!("jump to line {deep} (checkpoints built on the way): {:.1} ms", frame(&mut buffer, deep, at, vec![]));
    println!("same viewport again (cached): {:.1} ms", frame(&mut buffer, deep, at, vec![]));

    let mut caret = buffer.rope.line_to_char(deep + 10) + 4;
    let mut samples = Vec::new();
    for n in 0..1_000 {
        let started = Instant::now();
        let typed = ["c", "o", "n", "s", "t", " ", "{", "\n"][n % 8];
        buffer.insert(caret, typed);
        caret += typed.chars().count();
        if n % 50 == 49 {
            buffer.undo();
            caret -= 1;
        }
        let _ = frame(&mut buffer, deep, caret, vec![egui::Event::Text(typed.into())]);
        samples.push(started.elapsed().as_secs_f64() * 1e3);
    }
    let (p50, p95, p99) = (percentile(&mut samples, 0.5), percentile(&mut samples, 0.95), percentile(&mut samples, 0.99));
    println!("keystroke in the middle of 1 MB (insert + undo + highlight + layout + paint): p50 {p50:.2}  p95 {p95:.2}  p99 {p99:.2} ms");

    let mut samples = Vec::new();
    for n in 0..300 {
        let line = (n * 97) % (buffer.rope.len_lines() - 60);
        let started = Instant::now();
        let at = buffer.rope.line_to_char(line);
        frame(&mut buffer, line, at, vec![]);
        samples.push(started.elapsed().as_secs_f64() * 1e3);
    }
    println!("random jumps over the file: p50 {:.2}  p95 {:.2}  max {:.2} ms", percentile(&mut samples, 0.5), percentile(&mut samples, 0.95), percentile(&mut samples, 1.0));
}
