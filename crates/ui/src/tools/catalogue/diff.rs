//! Two texts compared, line by line or by their values: two modes over the same two texts.

mod structured;
mod text;

use egui::{Color32, CornerRadius, FontFamily, Margin, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use i18n::{Language, keys};

use crate::theme::palette;
use crate::tools::live::Turn;
use crate::tools::ui::note_picker::PickedNote;
use crate::tools::{Drawn, Tool, ToolAction, ToolResult};
use crate::widgets::{self, Segmented};

const ROW: f32 = 22.0;
const ROWS_MAX: f32 = 560.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Text,
    Structured,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    Split,
    Unified,
}

/// The two texts, and what they came from: a note's title, or nothing for what was typed.
#[derive(Default)]
struct Sides {
    a: String,
    b: String,
    a_name: Option<String>,
    b_name: Option<String>,
}

impl Sides {
    fn swap(&mut self) {
        std::mem::swap(&mut self.a, &mut self.b);
        std::mem::swap(&mut self.a_name, &mut self.b_name);
    }

    fn clear(&mut self) {
        *self = Self::default();
    }

    fn put(&mut self, slot: &str, note: &PickedNote) {
        let (text, name) = if slot == "a" {
            (&mut self.a, &mut self.a_name)
        } else {
            (&mut self.b, &mut self.b_name)
        };
        text.clone_from(&note.content);
        *name = Some(note.title.clone());
    }

    fn name_of(&self, side: &str, language: Language) -> String {
        let name = if side == "a" {
            &self.a_name
        } else {
            &self.b_name
        };
        name.clone()
            .unwrap_or_else(|| i18n::translate(language, keys::tools::diff::PASTED, &[]))
    }

    /// « A » or the note's title, for a note's own title.
    fn title_of(&self, side: &str) -> String {
        let name = if side == "a" {
            &self.a_name
        } else {
            &self.b_name
        };
        name.clone().unwrap_or_else(|| side.to_uppercase())
    }
}

pub(super) struct DiffTool {
    mode: Mode,
    sides: Sides,
    text: text::TextDiff,
    structured: structured::StructuredDiff,
}

pub(super) fn make() -> Box<dyn Tool> {
    Box::new(DiffTool {
        mode: Mode::Text,
        sides: Sides::default(),
        text: text::TextDiff::default(),
        structured: structured::StructuredDiff::default(),
    })
}

impl Tool for DiffTool {
    fn update(&mut self, turn: &mut Turn<'_>) {
        match self.mode {
            Mode::Text => self.text.update(&self.sides, turn),
            Mode::Structured => self.structured.update(&self.sides, turn),
        }
    }

    fn ui(&mut self, ui: &mut Ui, drawn: &mut Drawn<'_>) {
        let tr = |key| i18n::translate(drawn.language, key, &[]);
        let modes = [
            (Mode::Text, tr(keys::tools::diff::modes::TEXT)),
            (Mode::Structured, tr(keys::tools::diff::modes::STRUCTURED)),
        ];
        let modes = modes.each_ref().map(|(id, name)| (*id, name.as_str()));
        ui.spacing_mut().item_spacing.y = 12.0;
        Segmented::new(&modes, "diff-mode").show(ui, &mut self.mode);
        egui::ScrollArea::vertical()
            .id_salt("diff-page")
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                let asked = match self.mode {
                    Mode::Text => self.text.ui(ui, &mut self.sides, drawn),
                    Mode::Structured => self.structured.ui(ui, &mut self.sides, drawn),
                };
                if let Some(mode) = asked {
                    self.mode = mode;
                }
            });
    }

    fn keeps_result(&self) -> bool {
        true
    }

    fn result(&self, language: Language) -> Option<ToolResult> {
        match self.mode {
            Mode::Text => self.text.result(&self.sides, language),
            Mode::Structured => self.structured.result(&self.sides, language),
        }
    }

    fn imported(&mut self, slot: &str, note: &PickedNote) {
        self.sides.put(slot, note);
    }

    fn actions(&self) -> Vec<ToolAction> {
        let swap = ToolAction {
            id: "swap",
            label: keys::tools::diff::SWAP,
            enabled: true,
        };
        let copy = match self.mode {
            Mode::Text => ToolAction {
                id: "copy-unified",
                label: keys::tools::diff::text::COPY_UNIFIED,
                enabled: self.text.unified().is_some(),
            },
            Mode::Structured => ToolAction {
                id: "copy-patch",
                label: keys::tools::diff::structured::COPY_PATCH,
                enabled: self.structured.patch().is_some(),
            },
        };
        vec![swap, copy]
    }

    fn run_action(&mut self, id: &str, ctx: &egui::Context) {
        let copied = match id {
            "swap" => {
                self.sides.swap();
                None
            }
            "copy-unified" => self.text.unified(),
            "copy-patch" => self.structured.patch(),
            _ => None,
        };
        if let Some(copied) = copied {
            ctx.copy_text(copied.to_owned());
        }
    }

    fn has_sample(&self) -> bool {
        true
    }

    fn sample(&mut self, _: Language) {
        match self.mode {
            Mode::Text => text::fill_sample(&mut self.sides),
            Mode::Structured => self.structured.fill_sample(&mut self.sides),
        }
    }

    fn clear(&mut self) {
        self.sides.clear();
        self.structured.forget_selection();
    }
}

/// How a row is lit, whichever mode drew it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Same,
    Added,
    Removed,
    Modified,
}

/// Its tint, and the colour of its mark.
fn lit(kind: Kind, ui: &Ui) -> (Color32, Color32) {
    let p = palette(ui.ctx());
    match kind {
        Kind::Same => (Color32::TRANSPARENT, p.text_2),
        Kind::Added => (p.green.gamma_multiply(0.12), p.green),
        Kind::Removed => (p.red.gamma_multiply(0.12), p.red),
        Kind::Modified => (p.amber_fill.gamma_multiply(0.08), p.amber_text),
    }
}

fn mark_of(kind: Kind, right: bool) -> &'static str {
    match kind {
        Kind::Same => "",
        Kind::Modified => "~",
        Kind::Added | Kind::Removed if right => "+",
        Kind::Added | Kind::Removed => "-",
    }
}

/// A side with no line: hatched, so the eye reads it as absent rather than blank.
fn gap(ui: &Ui, rect: Rect) {
    let p = palette(ui.ctx());
    ui.painter().rect_filled(rect, 0.0, p.bg_1);
    let painter = ui.painter().with_clip_rect(rect);
    let mut x = rect.left() - rect.height();
    while x < rect.right() {
        let from = Pos2::new(x, rect.bottom());
        let to = Pos2::new(x + rect.height(), rect.top());
        painter.line_segment([from, to], Stroke::new(1.0, p.bg_0));
        x += 12.0;
    }
}

/// The mark and the line number at the head of a cell: what is left for the code.
fn cell_head(ui: &Ui, rect: Rect, kind: Kind, right: bool, number: u32) -> Rect {
    let p = palette(ui.ctx());
    let (tint, ink) = lit(kind, ui);
    ui.painter().rect_filled(rect, 0.0, tint);
    let middle = rect.center().y;
    let mark = mark_of(kind, right);
    if !mark.is_empty() {
        let mark = widgets::text(ui, mark, 12.0, 700.0, FontFamily::Monospace, ink);
        let at = Pos2::new(
            rect.left() + 10.0 - mark.size().x / 2.0,
            middle - mark.size().y / 2.0,
        );
        ui.painter().galley(at, mark, ink);
    }
    let number = number.to_string();
    let number = widgets::text(ui, &number, 12.0, 400.0, FontFamily::Monospace, p.line_no);
    let at = Pos2::new(
        rect.left() + 44.0 - number.size().x,
        middle - number.size().y / 2.0,
    );
    ui.painter().galley(at, number, p.line_no);
    Rect::from_min_max(Pos2::new(rect.left() + 54.0, rect.top()), rect.max)
}

/// The rows' box, with its A and B over the two halves when `head` says.
fn rows_box(ui: &mut Ui, author_id: &str, height: f32, content: impl FnOnce(&mut Ui)) {
    let p = palette(ui.ctx());
    let shown = egui::Frame::new()
        .fill(p.bg_0)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::ZERO)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            ui.set_max_height(height.min(ROWS_MAX));
            content(ui);
        });
    widgets::address(ui, &shown.response, author_id);
}

fn split_head(ui: &mut Ui, sides: &Sides, language: Language) {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 34.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius {
            nw: 10,
            ne: 10,
            ..CornerRadius::ZERO
        },
        p.bg_1,
    );
    ui.painter()
        .hline(rect.x_range(), rect.bottom() - 0.5, (1.0, p.line_soft));
    ui.painter()
        .vline(rect.center().x, rect.y_range(), (1.0, p.line_soft));
    for (side, left) in [("a", rect.left()), ("b", rect.center().x)] {
        let words = format!("{}  {}", side.to_uppercase(), sides.name_of(side, language));
        let galley = widgets::one_line(ui, &words, (12.0, 400.0), p.text_0, width / 2.0 - 24.0);
        let at = Pos2::new(left + 12.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, p.text_0);
    }
}

/// A count in its pill.
fn pill(ui: &mut Ui, words: &str, hue: Color32, author_id: &str) {
    let galley = widgets::text(ui, words, 11.5, 400.0, FontFamily::Monospace, hue);
    let (rect, response) =
        ui.allocate_exact_size(galley.size() + Vec2::new(20.0, 8.0), Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, words));
    widgets::address(ui, &response, author_id);
    ui.painter()
        .rect_filled(rect, CornerRadius::same(12), hue.gamma_multiply(0.14));
    ui.painter()
        .galley(rect.min + Vec2::new(10.0, 4.0), galley, hue);
}

/// What a row's text would be were the two sides one above the other.
fn unified_height(rows: usize) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "rows on a screen")]
    let height = rows as f32 * ROW;
    height + 8.0
}
