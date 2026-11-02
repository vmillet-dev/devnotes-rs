//! A minimal `RichEditor` view: one galley per text block, a caret that crosses blocks with
//! the arrows (by rows inside a block, keeping its x across blocks), typing, and input rules.

use std::sync::Arc;

use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{Color32, FontId, Galley, Key, Pos2, vec2};

use crate::model::{Block, Inline, Marks, input_rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caret {
    pub block: usize,
    pub offset: usize,
}

pub struct RichView {
    pub blocks: Vec<Block>,
    pub caret: Caret,
    galleys: Vec<(Pos2, Arc<Galley>)>,
}

fn job(block: &Block, width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let (size, indent) = match block {
        Block::Heading(level, _) => (26.0 - 3.0 * f32::from(*level), 0.0),
        Block::List { .. } => (14.0, 20.0),
        _ => (14.0, 0.0),
    };
    let runs: &[Inline] = match block {
        Block::Paragraph(r) | Block::Heading(_, r) => r,
        _ => &[],
    };
    for run in runs {
        if let Inline::Text { text, marks, .. } = run {
            let mut format = TextFormat::simple(if marks.code { FontId::monospace(13.0) } else { FontId::proportional(size) }, Color32::from_gray(225));
            format.italics = marks.italic;
            if marks.bold {
                format.color = Color32::WHITE;
            }
            job.append(text, 0.0, format);
        }
    }
    if let Block::Code { text, .. } = block {
        job.append(text, 0.0, TextFormat::simple(FontId::monospace(13.0), Color32::from_gray(200)));
    }
    if job.text.is_empty() {
        job.append("", indent, TextFormat::simple(FontId::proportional(size), Color32::WHITE));
    }
    job.wrap.max_width = width - indent;
    job
}

impl RichView {
    pub fn new(blocks: Vec<Block>) -> Self {
        // Lists are flattened to one text block per item for the prototype.
        let flat = blocks
            .into_iter()
            .flat_map(|b| match b {
                Block::List { items, .. } => items.into_iter().flat_map(|i| i.blocks).collect(),
                other => vec![other],
            })
            .collect();
        Self { blocks: flat, caret: Caret { block: 0, offset: 0 }, galleys: Vec::new() }
    }

    fn len(&self, block: usize) -> usize {
        self.blocks[block].text().chars().count()
    }

    fn type_text(&mut self, typed: &str) {
        let Caret { block, offset } = self.caret;
        if let Block::Paragraph(runs) | Block::Heading(_, runs) = &mut self.blocks[block] {
            let mut text: String = Block::Paragraph(runs.clone()).text();
            let at = text.char_indices().nth(offset).map_or(text.len(), |(i, _)| i);
            text.insert_str(at, typed);
            *runs = vec![Inline::Text { text, marks: Marks::default(), link: None }];
            self.caret.offset += typed.chars().count();
        }
        if let Some(ruled) = input_rule(&self.blocks[block]) {
            let before = self.len(block);
            self.blocks[block] = match ruled {
                Block::List { items, .. } => items.into_iter().next().and_then(|i| i.blocks.into_iter().next()).expect("item"),
                other => other,
            };
            self.caret.offset = self.caret.offset.saturating_sub(before - self.len(block));
        }
    }

    fn vertical(&mut self, down: bool) {
        let Some((origin, galley)) = self.galleys.get(self.caret.block).cloned() else { return };
        let here = galley.pos_from_cursor(CCursor::new(self.caret.offset));
        let x = here.min.x;
        let row_y = if down { here.max.y + 1.0 } else { here.min.y - 1.0 };
        if row_y >= 0.0 && row_y <= galley.rect.height() {
            self.caret.offset = galley.cursor_from_pos(vec2(x, row_y)).index.into();
            return;
        }
        let next = if down { self.caret.block + 1 } else { self.caret.block.wrapping_sub(1) };
        if let Some((_, target)) = self.galleys.get(next) {
            let y = if down { 1.0 } else { target.rect.height() - 1.0 };
            self.caret = Caret { block: next, offset: target.cursor_from_pos(vec2(x, y)).index.into() };
        }
        let _ = origin;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let (left, right, up, down) = ui.input(|i| (i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)));
        let typed: String = ui.input(|i| i.events.iter().filter_map(|e| if let egui::Event::Text(t) = e { Some(t.clone()) } else { None }).collect());
        if !typed.is_empty() {
            self.type_text(&typed);
        }
        if right {
            if self.caret.offset < self.len(self.caret.block) {
                self.caret.offset += 1;
            } else if self.caret.block + 1 < self.blocks.len() {
                self.caret = Caret { block: self.caret.block + 1, offset: 0 };
            }
        }
        if left {
            if self.caret.offset > 0 {
                self.caret.offset -= 1;
            } else if self.caret.block > 0 {
                self.caret = Caret { block: self.caret.block - 1, offset: self.len(self.caret.block - 1) };
            }
        }
        if up {
            self.vertical(false);
        }
        if down {
            self.vertical(true);
        }
        let width = ui.available_width();
        let mut y = ui.min_rect().top();
        let x = ui.min_rect().left();
        self.galleys.clear();
        for block in &self.blocks {
            let galley = ui.fonts_mut(|f| f.layout_job(job(block, width)));
            self.galleys.push((Pos2::new(x, y), galley.clone()));
            ui.painter().galley(Pos2::new(x, y), galley.clone(), Color32::WHITE);
            y += galley.rect.height() + 8.0;
        }
        if let Some((origin, galley)) = self.galleys.get(self.caret.block) {
            let at = galley.pos_from_cursor(CCursor::new(self.caret.offset)).translate(origin.to_vec2());
            ui.painter().vline(at.min.x, at.y_range(), egui::Stroke::new(2.0, Color32::from_rgb(232, 168, 56)));
        }
    }
}
