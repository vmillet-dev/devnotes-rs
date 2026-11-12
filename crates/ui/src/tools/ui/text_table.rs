//! A small table of texts, its first column in the mono face. A row can be chosen.

use egui::{CornerRadius, FontFamily, Pos2, Role, Sense, Stroke, Ui, Vec2, WidgetInfo};

use crate::theme::palette;
use crate::widgets::{self, address, hovered, interact, towards};

const ROW: f32 = 28.0;

/// A heading, and its share of the width.
pub struct TextColumn {
    label: String,
    share: f32,
}

impl TextColumn {
    pub fn new(label: String, share: f32) -> Self {
        Self { label, share }
    }
}

/// The row chosen this frame. `lit` is the one that stands for what is on screen.
pub fn text_table(
    ui: &mut Ui,
    author_id: &str,
    columns: &[TextColumn],
    rows: &[Vec<String>],
    lit: Option<usize>,
) -> Option<usize> {
    let p = palette(ui.ctx());
    let mut chosen = None;
    let shown = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(10))
        .show(ui, |ui| {
            let width = ui.available_width();
            ui.set_width(width);
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            head(ui, columns, width);
            for (at, cells) in rows.iter().enumerate() {
                let id = format!("{author_id}-row-{at}");
                if row(ui, (&id, columns), cells, lit == Some(at)).clicked() {
                    chosen = Some(at);
                }
            }
        });
    address(ui, &shown.response, author_id);
    chosen
}

fn head(ui: &mut Ui, columns: &[TextColumn], width: f32) {
    let p = palette(ui.ctx());
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::hover());
    let mut left = rect.left() + 12.0;
    for column in columns {
        let words = column.label.to_uppercase();
        let room = width * column.share - 8.0;
        let galley = widgets::one_line(ui, &words, (10.5, 600.0), p.text_2, room);
        let at = Pos2::new(left, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(at, galley, p.text_2);
        left += width * column.share;
    }
}

fn row(
    ui: &mut Ui,
    (author_id, columns): (&str, &[TextColumn]),
    cells: &[String],
    lit: bool,
) -> egui::Response {
    let p = palette(ui.ctx());
    let width = ui.available_width();
    let first = cells.first().map_or("", String::as_str);
    let response = interact(
        ui,
        Vec2::new(width, ROW),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::Row, true, lit, first),
        CornerRadius::ZERO,
    );
    let rect = response.rect;
    let fill = if lit {
        p.amber_fill.gamma_multiply(0.12)
    } else {
        towards(egui::Color32::TRANSPARENT, p.bg_2, hovered(ui, &response))
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    ui.painter()
        .hline(rect.x_range(), rect.top() + 0.5, (1.0, p.line_soft));
    let mut left = rect.left() + 12.0;
    for (column, cell) in columns.iter().zip(cells) {
        let family = FontFamily::Monospace;
        let galley = widgets::text(ui, cell, 12.0, 400.0, family, p.text_0);
        let at = Pos2::new(left, rect.center().y - galley.size().y / 2.0);
        ui.painter()
            .with_clip_rect(rect)
            .galley(at, galley, p.text_0);
        left += width * column.share;
    }
    response
}
