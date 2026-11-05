//! The drawings of the Notes' `mermaid` blocks (plan § 3.9.3): drawn off the frame one at a time,
//! uploaded, the last 64 kept, so a Note reopened or a keystroke undone draws at once. They are
//! library content: they live in `LibraryCaches`.

use std::collections::{HashMap, VecDeque};

use egui::{Color32, ColorImage, Context, TextureOptions, Theme};

use crate::api;
use crate::backend::{Backend, Pending};
use crate::theme::tokens::{DARK, LIGHT, Palette};
use crate::widgets::rich::Diagram;

const KEPT: usize = 64;

/// A source in the theme it is drawn in: dark or not.
type Key = (bool, String);

enum Entry {
    Queued,
    Drawing(Pending<diagram::Drawn>),
    Done(Diagram),
}

#[derive(Default)]
pub struct Diagrams {
    entries: HashMap<Key, Entry>,
    /// Asked, oldest first: drawn one at a time.
    queue: VecDeque<Key>,
    /// Oldest first: what goes when more than `KEPT` are done.
    done: VecDeque<Key>,
}

impl Diagrams {
    /// The drawing of `source` in the theme on screen; `None` while there is none.
    pub fn get(&self, ctx: &Context, source: &str) -> Option<Diagram> {
        match self.entries.get(&key(ctx, source))? {
            Entry::Done(drawn) => Some(drawn.clone()),
            Entry::Queued | Entry::Drawing(_) => None,
        }
    }

    pub fn ask(&mut self, ctx: &Context, source: &str) {
        let key = key(ctx, source);
        if self.entries.contains_key(&key) {
            return;
        }
        self.entries.insert(key.clone(), Entry::Queued);
        self.queue.push_back(key);
    }

    /// Once a frame: a drawing finished is uploaded, and the next one starts.
    pub fn update(&mut self, ctx: &Context, backend: &Backend) {
        let mut drawing = false;
        for (key, entry) in &mut self.entries {
            let Entry::Drawing(pending) = entry else {
                continue;
            };
            match pending.poll() {
                Some(answer) => {
                    let drawn = answer.unwrap_or(diagram::Drawn::Unsupported);
                    *entry = Entry::Done(shown(ctx, drawn));
                    self.done.push_back(key.clone());
                }
                None => drawing = true,
            }
        }
        if !drawing && let Some(key) = self.queue.pop_front() {
            let theme = theme_of(if key.0 { &DARK } else { &LIGHT }, key.0);
            let pending = api::notes::draw_diagram(backend, key.1.clone(), theme);
            self.entries.insert(key, Entry::Drawing(pending));
        }
        while self.done.len() > KEPT {
            if let Some(oldest) = self.done.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }
}

fn key(ctx: &Context, source: &str) -> Key {
    (ctx.theme() == Theme::Dark, source.to_owned())
}

fn shown(ctx: &Context, drawn: diagram::Drawn) -> Diagram {
    match drawn {
        diagram::Drawn::Picture {
            width,
            height,
            pixels,
        } => {
            let size = [width as usize, height as usize];
            let image = ColorImage::from_rgba_premultiplied(size, &pixels);
            Diagram::Drawn(ctx.load_texture("diagram", image, TextureOptions::LINEAR))
        }
        diagram::Drawn::Failed { line, detail } => Diagram::Failed { line, detail },
        diagram::Drawn::Unsupported => Diagram::Unsupported,
    }
}

/// The tokens Mermaid derives its shades from, as the reference's `MermaidRenderer` gave them.
fn theme_of(p: &Palette, dark: bool) -> diagram::Theme {
    diagram::Theme {
        dark,
        background: hex(p.bg_1),
        primary: hex(p.bg_3),
        primary_text: hex(p.text_0),
        primary_border: hex(p.line),
        secondary: hex(p.bg_2),
        line: hex(p.text_2),
        text: hex(p.text_0),
    }
}

fn hex(colour: Color32) -> String {
    let [r, g, b, _] = colour.to_array();
    format!("#{r:02x}{g:02x}{b:02x}")
}
