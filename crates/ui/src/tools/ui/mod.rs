//! What the tools are built from: their two columns, their fields and the rows of their results.

mod copy_value;
mod note_import;
pub mod note_picker;
mod output_row;
mod reference_table;
mod result_row;
mod text_table;

pub use copy_value::CopyValue;
pub use note_import::{note_import, note_import_into};
pub use output_row::OutputRow;
pub use reference_table::{Cell, Column, Group, ReferenceRow, ReferenceTable};
pub use result_row::{ResultRow, result_rows};
pub use text_table::{TextColumn, text_table};

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Id, Key, Margin, Modifiers, Response, Role, Sense,
    Stroke, TextEdit, Ui, UiBuilder, Vec2, WidgetInfo,
};

use super::Drawn;
use crate::theme::palette;
use crate::widgets::{self, address};

/// Under this window width, one column and the page scrolling.
const NARROW: f32 = 960.0;
const COLUMN_GAP: f32 = 18.0;
/// The results' share against the input's.
const RESULTS_SHARE: f32 = 1.15;

/// A tool's column, drawn by a method of its own.
pub type Part<T> = fn(&mut T, &mut Ui, &mut Drawn<'_>);

/// What goes in on the left, what comes out on the right, each scrolling on its own.
pub fn columns<T>(
    ui: &mut Ui,
    tool: &mut T,
    drawn: &mut Drawn<'_>,
    (left, right): (Part<T>, Part<T>),
) {
    if ui.ctx().content_rect().width() < NARROW {
        egui::ScrollArea::vertical()
            .id_salt("tool-column")
            .auto_shrink(false)
            .show(ui, |ui| {
                left(tool, ui, drawn);
                ui.add_space(COLUMN_GAP);
                right(tool, ui, drawn);
            });
        return;
    }
    let area = ui.max_rect();
    let unit = (area.width() - COLUMN_GAP) / (1.0 + RESULTS_SHARE);
    let (first, rest) = area.split_left_right_at_x(area.left() + unit);
    let second = rest.with_min_x(first.right() + COLUMN_GAP);
    for (rect, salt, part) in [(first, "tool-input", left), (second, "tool-output", right)] {
        let mut column = ui.new_child(UiBuilder::new().id_salt(salt).max_rect(rect));
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(&mut column, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                part(tool, ui, drawn);
            });
    }
}

/// A section's title inside a tool or the tools' home: small capitals, muted.
pub fn section_title(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    let shown = words.to_uppercase();
    let galley = widgets::text(ui, &shown, 11.0, 700.0, FontFamily::Proportional, p.text_2);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Heading, true, words));
    ui.painter().galley(rect.min, galley, p.text_2);
}

/// A section's title with a control at its other end: « INPUT … Text | File ».
pub fn section_head(ui: &mut Ui, words: &str, end: impl FnOnce(&mut Ui)) {
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(width, 28.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(Vec2::new(width, 28.0));
            section_title(ui, words);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), end);
        },
    );
}

/// A label over a control: « Detected words ».
pub fn label(ui: &mut Ui, words: &str) -> Response {
    let ink = palette(ui.ctx()).text_2;
    paragraph(ui, words, (11.5, 500.0), ink)
}

/// A hint under a control.
pub fn hint(ui: &mut Ui, words: &str) -> Response {
    let ink = palette(ui.ctx()).text_2;
    paragraph(ui, words, (11.0, 400.0), ink)
}

/// What went wrong with an input, drawn where the answer would be.
pub fn problem(ui: &mut Ui, words: &str, author_id: &str) -> Response {
    let ink = palette(ui.ctx()).red;
    let said = paragraph(ui, words, (12.0, 400.0), ink);
    address(ui, &said, author_id);
    said
}

/// Words wrapped to the room there is.
pub fn paragraph(ui: &mut Ui, words: &str, style: (f32, f32), ink: Color32) -> Response {
    let shown = widgets::wrapped(ui, words, style, ink, ui.available_width());
    let (rect, response) = ui.allocate_exact_size(shown.size(), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, words));
    shown.paint(ui, rect.min, ink);
    response
}

/// The text of `value`, an enum of the core read as it is written, under `prefix`.
pub fn named(language: i18n::Language, prefix: &str, value: &serde_json::Value) -> String {
    let path = format!("{prefix}.{}", value.as_str().unwrap_or_default());
    i18n::Key::find(&path).map_or_else(String::new, |key| i18n::translate(language, key, &[]))
}

/// A checkbox and its label, in the accent.
pub fn check(ui: &mut Ui, value: &mut bool, words: &str, author_id: &str) -> Response {
    let p = palette(ui.ctx());
    let label = widgets::text(ui, words, 12.0, 400.0, FontFamily::Proportional, p.text_1);
    let on = *value;
    let response = widgets::interact(
        ui,
        Vec2::new(14.0 + 7.0 + label.size().x, 24.0),
        Sense::click(),
        author_id,
        || WidgetInfo::selected(Role::CheckBox, true, on, words),
        CornerRadius::same(4),
    );
    if response.clicked() {
        *value = !*value;
    }
    let rect = response.rect;
    let tick = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - 7.0),
        Vec2::splat(14.0),
    );
    let corner = CornerRadius::same(3);
    if *value {
        ui.painter().rect_filled(tick, corner, p.amber_fill);
        crate::theme::icons::paint(
            ui,
            tick.shrink(1.5),
            crate::theme::icons::Icon::Check,
            p.amber_ink,
        );
    } else {
        let edge = Stroke::new(1.0, p.line);
        ui.painter()
            .rect(tick, corner, p.bg_1, edge, egui::StrokeKind::Inside);
    }
    let at = egui::pos2(tick.right() + 7.0, rect.center().y - label.size().y / 2.0);
    ui.painter().galley(at, label, p.text_1);
    response
}

/// A count typed by hand: the tool reads it when it parses.
pub fn number_field(ui: &mut Ui, value: &mut String, name: &str, author_id: &str) -> Response {
    ui.scope(|ui| {
        ui.set_max_width(72.0);
        widgets::TextField::new(value, name, author_id)
            .compact()
            .show(ui)
    })
    .inner
}

/// What is typed or pasted into a tool. `rows` is how many lines it opens on.
pub fn text_area(
    ui: &mut Ui,
    value: &mut String,
    (name, author_id): (&str, &str),
    hint: &str,
    rows: usize,
) -> Response {
    let p = palette(ui.ctx());
    let id = ui.next_auto_id().with(author_id);
    let focused = ui.memory(|memory| memory.has_focus(id));
    let edge = if focused { p.amber_edge } else { p.line };
    egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, edge))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            let response = ui.add(
                TextEdit::multiline(value)
                    .id(id)
                    .frame(egui::Frame::new())
                    .hint_text(hint)
                    .text_color(p.text_0)
                    .font(FontId::new(12.5, FontFamily::Monospace))
                    .desired_rows(rows)
                    .desired_width(f32::INFINITY),
            );
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_label(name.to_string()));
            address(ui, &response, author_id);
            response
        })
        .inner
}

/// A card of results, its rows under small group titles.
pub fn card<R>(ui: &mut Ui, author_id: &str, rows: impl FnOnce(&mut Ui) -> R) -> R {
    let p = palette(ui.ctx());
    let shown = egui::Frame::new()
        .fill(p.bg_1)
        .stroke(Stroke::new(1.0, p.line_soft))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin {
            bottom: 4,
            ..Margin::ZERO
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            rows(ui)
        });
    address(ui, &shown.response, author_id);
    shown.inner
}

/// « For code », « For text »: a label over the rows of a card.
pub fn group_title(ui: &mut Ui, words: &str) {
    let p = palette(ui.ctx());
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        paragraph(ui, words, (11.5, 500.0), p.text_2);
    });
    ui.add_space(6.0);
}

/// The arrows walk `results` in the order they are read, the first one up being `field`; egui
/// would walk them by position.
pub(super) fn walk(ui: &Ui, results: &[Id], field: &Response) {
    let pressed = |key| ui.input_mut(|input| input.consume_key(Modifiers::NONE, key));
    if widgets::takes_keys(ui, field) {
        if let Some(id) = results.first()
            && pressed(Key::ArrowDown)
        {
            ui.memory_mut(|memory| memory.request_focus(*id));
        }
        return;
    }
    let focused = ui.memory(egui::Memory::focused);
    let Some(at) = focused.and_then(|id| results.iter().position(|it| *it == id)) else {
        return;
    };
    let filter = egui::EventFilter {
        vertical_arrows: true,
        ..egui::EventFilter::default()
    };
    ui.memory_mut(|memory| memory.set_focus_lock_filter(results[at], filter));
    if pressed(Key::ArrowDown)
        && let Some(id) = results.get(at + 1)
    {
        ui.memory_mut(|memory| memory.request_focus(*id));
    }
    if pressed(Key::ArrowUp) {
        let up = at.checked_sub(1).map_or(field.id, |it| results[it]);
        ui.memory_mut(|memory| memory.request_focus(up));
    }
}

#[cfg(test)]
mod tests;
